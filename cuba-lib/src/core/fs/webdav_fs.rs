use chrono::{DateTime, Utc};
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, percent_encode};
use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use reqwest::blocking::RequestBuilder;
use reqwest::{Method, Url};
use secrecy::{ExposeSecret, SecretString};
use std::io::{Read, pipe};
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;
use unicode_normalization::UnicodeNormalization;
use url::ParseError;
use warned::Warned;

use crate::shared::npath::{
    Abs, Dir, File, NPath, NPathComponent, NPathError, NPathRoot, Rel, UNPath,
};

use super::fs_base::{FS, FSBlockSize, FSError, FSWrite};
use super::fs_node::{FSNode, FSNodeMetaData};

fn parse_rfc1123(input: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    const RFC1123: &str = "%a, %d %b %Y %H:%M:%S %z";

    // Replace "UTC" and "GMT" with 0000
    let normalized = input.trim().replace("UTC", "+0000").replace("GMT", "+0000");

    // Parse using the RFC 1123 format string
    DateTime::parse_from_str(&normalized, RFC1123).map(|dt| dt.with_timezone(&Utc))
}

/// Parse datetime.
fn parse_webdav_datetime(string: &str) -> Option<SystemTime> {
    // Try RFC 1123
    if let Ok(dt) = parse_rfc1123(string) {
        return Some(SystemTime::from(dt.with_timezone(&Utc)));
    }

    // Try RFC 3339
    if let Ok(dt) = DateTime::parse_from_rfc3339(string) {
        return Some(SystemTime::from(dt));
    }

    // Try RFC 2822
    if let Ok(dt) = DateTime::parse_from_rfc2822(string) {
        return Some(SystemTime::from(dt.with_timezone(&Utc)));
    }

    None
}

/// Chooses the last valid path for error output.
pub fn choose_path(abs_path: &UNPath<Abs>, entry_rel_path: &Option<UNPath<Rel>>) -> UNPath<Abs> {
    if let Some(entry_rel_path) = entry_rel_path {
        match abs_path {
            UNPath::File(_file_path) => abs_path.clone(),
            UNPath::Dir(dir_path) => match dir_path.union(entry_rel_path) {
                Ok(dir_path) => dir_path,
                Err(_err) => abs_path.clone(),
            },
        }
    } else {
        abs_path.clone()
    }
}

/// Make url from abs path.
pub fn make_url_from_abs(abs_path: &UNPath<Abs>) -> Result<Url, ParseError> {
    let mut path = String::new();

    for component in abs_path.components() {
        match component {
            NPathComponent::Root(NPathRoot::UrlScheme(scheme)) => {
                // scheme already contains e.g. "http:" or "webdav:"
                path.push_str(&scheme);
                path.push('/');
            }
            NPathComponent::Normal(segment) => {
                // Normalize to NFC to ensure stable unicode encoding.
                let normalized = segment.nfc().collect::<String>();

                // Percent-encode after normalization.
                let encoded = percent_encode(normalized.as_bytes(), NON_ALPHANUMERIC);

                path.push_str(encoded.to_string().as_str());
                path.push('/');
            }
            // ignore other roots; probably shouldn't happen.
            _ => {}
        }
    }

    // Remove trailing slash if file.
    if path.ends_with('/') {
        path.pop();
    }

    Url::parse(&path)
}

/// Make rel path from encoded str path.
pub fn make_rel_path_from_str_path(path: &str) -> Result<UNPath<Rel>, NPathError> {
    let decoded_path = percent_decode_str(path).decode_utf8_lossy().to_string();

    // Path must be relative.
    let rel_path = decoded_path.trim_start_matches("/").to_string();

    if rel_path.ends_with("/") {
        Ok(UNPath::Dir(NPath::<Rel, Dir>::try_from(rel_path.as_str())?))
    } else {
        Ok(UNPath::File(NPath::<Rel, File>::try_from(
            rel_path.as_str(),
        )?))
    }
}

#[derive(PartialEq)]
enum Context {
    Response,
    Href,
    Propstat,
    Prop,
    Resourcetype,
    Collection,
    Getcontentlength,
    Creationdate,
    Getlastmodified,
}

pub struct WebDAVFS {
    username: String,
    password: SecretString,
    timeout_secs: u64,
    client: reqwest::blocking::Client,
    connected: bool,
}

impl WebDAVFS {
    pub fn new(username: &str, password: &SecretString, timeout_secs: u64) -> Self {
        WebDAVFS {
            username: username.to_owned(),
            password: password.to_owned(),
            timeout_secs,
            client: reqwest::blocking::Client::new(),
            connected: false,
        }
    }

    fn start_request(&self, method: Method, url: &Url) -> RequestBuilder {
        self.client
            .request(method, url.clone())
            .basic_auth(self.username.as_str(), Some(self.password.expose_secret()))
    }

    fn get_file_size_with_range(&self, abs_path: &UNPath<Abs>) -> Result<u64, FSError> {
        match make_url_from_abs(abs_path) {
            Ok(url) => {
                let response = self
                    .start_request(Method::GET, &url)
                    .header("Range", "bytes=0-0")
                    .send()
                    .map_err(|err| FSError::MetaFailed(abs_path.clone(), err.into()))?;

                if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
                    return Err(FSError::MetaFailed(
                        abs_path.clone(),
                        "Status code must be partial-content".into(),
                    ));
                }

                if let Some(content_range) = response.headers().get("Content-Range") {
                    let value = content_range
                        .to_str()
                        .map_err(|err| FSError::MetaFailed(abs_path.clone(), err.into()))?;
                    // Expected format: bytes 0-0/123456
                    if let Some(total_str) = value.split('/').nth(1) {
                        total_str
                            .parse::<u64>()
                            .map_err(|err| FSError::MetaFailed(abs_path.clone(), err.into()))
                    } else {
                        Err(FSError::MetaFailed(
                            abs_path.clone(),
                            "Wrong total bytes format".into(),
                        ))
                    }
                } else {
                    Err(FSError::MetaFailed(
                        abs_path.clone(),
                        "Content-range is None".into(),
                    ))
                }
            }
            Err(err) => Err(FSError::MetaFailed(abs_path.clone(), err.into())),
        }
    }

    fn parse_response(
        &self,
        abs_path: &UNPath<Abs>,
        include_path: bool,
        xml: &str,
    ) -> Result<Vec<FSNode>, FSError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        reader.config_mut().expand_empty_elements = true;

        let mut fs_nodes: Vec<FSNode> = Vec::new();

        let mut xml_buf = Vec::new();
        let mut context: Vec<Context> = Vec::new();

        let mut entry_rel_path: Option<UNPath<Rel>> = None;
        let mut fs_metadata: Option<FSNodeMetaData> = None;
        let mut is_dir: Option<bool> = None;
        let mut size: Option<u64> = None;
        let mut created: Option<SystemTime> = None;
        let mut modified: Option<SystemTime> = None;
        let mut href_buf = String::new();

        while let Ok(event) = reader.read_event_into(&mut xml_buf) {
            match event {
                Event::Start(ref element) => {
                    let name = element.name();
                    let local_name = name.local_name();
                    let local_name_ref = local_name.as_ref();

                    match local_name_ref {
                        b"response" if context.is_empty() => {
                            entry_rel_path = None;

                            context.push(Context::Response);
                        }
                        b"href" if context.last() == Some(&Context::Response) => {
                            context.push(Context::Href);

                            href_buf.clear();
                        }
                        b"propstat" if context.last() == Some(&Context::Response) => {
                            fs_metadata = None;

                            context.push(Context::Propstat);
                        }
                        b"prop" if context.last() == Some(&Context::Propstat) => {
                            is_dir = None;
                            size = None;
                            created = None;
                            modified = None;

                            context.push(Context::Prop);
                        }
                        b"resourcetype" if context.last() == Some(&Context::Prop) => {
                            is_dir = Some(false);

                            context.push(Context::Resourcetype);
                        }
                        b"collection" if context.last() == Some(&Context::Resourcetype) => {
                            is_dir = Some(true);

                            context.push(Context::Collection);
                        }
                        b"getcontentlength" if context.last() == Some(&Context::Prop) => {
                            context.push(Context::Getcontentlength);
                        }
                        b"creationdate" if context.last() == Some(&Context::Prop) => {
                            context.push(Context::Creationdate);
                        }
                        b"getlastmodified" if context.last() == Some(&Context::Prop) => {
                            context.push(Context::Getlastmodified);
                        }
                        _ => {}
                    }
                }
                Event::End(ref element) => {
                    let name = element.name();
                    let local_name = name.local_name();
                    let local_name_ref = local_name.as_ref();

                    match local_name_ref {
                        b"response" if context.last() == Some(&Context::Response) => {
                            if let (Some(is_dir), Some(entry_rel_path), Some(fs_metadata)) =
                                (is_dir, entry_rel_path.clone(), fs_metadata.clone())
                            {
                                // Path target must be the same.
                                if is_dir != entry_rel_path.is_dir() {
                                    return Err(FSError::MetaFailed(
                                        choose_path(abs_path, &Some(entry_rel_path.clone())),
                                        "Path target mismatch".into(),
                                    ));
                                }

                                let entry_abs_path: UNPath<Abs> = match abs_path {
                                    UNPath::File(_file_path) => abs_path.clone(),
                                    UNPath::Dir(dir_path) => {
                                        dir_path.union(&entry_rel_path).map_err(|err| {
                                            FSError::MetaFailed(
                                                choose_path(abs_path, &None),
                                                err.into(),
                                            )
                                        })?
                                    }
                                };

                                let fs_node = FSNode {
                                    abs_path: entry_abs_path.clone(),
                                    metadata: fs_metadata,
                                };

                                if include_path || *abs_path != entry_abs_path {
                                    fs_nodes.push(fs_node);
                                }
                            }

                            context.pop();
                        }
                        b"href" if context.last() == Some(&Context::Href) => {
                            context.pop();

                            entry_rel_path =
                                Some(make_rel_path_from_str_path(&href_buf).map_err(|err| {
                                    FSError::MetaFailed(choose_path(abs_path, &None), err.into())
                                })?);
                        }
                        b"propstat" if context.last() == Some(&Context::Propstat) => {
                            if let (Some(is_dir), Some(created), Some(modified)) =
                                (is_dir, created, modified)
                            {
                                if !is_dir && size.is_none() {
                                    return Err(FSError::MetaFailed(
                                        choose_path(abs_path, &entry_rel_path),
                                        "File size is zero".into(),
                                    ));
                                } else {
                                    fs_metadata = Some(FSNodeMetaData {
                                        created,
                                        modified,
                                        size: size.unwrap_or(0),
                                    });
                                }
                            }

                            context.pop();
                        }
                        b"prop" if context.last() == Some(&Context::Prop) => {
                            context.pop();
                        }
                        b"resourcetype" if context.last() == Some(&Context::Resourcetype) => {
                            context.pop();
                        }
                        b"collection" if context.last() == Some(&Context::Collection) => {
                            context.pop();
                        }
                        b"getcontentlength"
                            if context.last() == Some(&Context::Getcontentlength) =>
                        {
                            context.pop();
                        }
                        b"creationdate" if context.last() == Some(&Context::Creationdate) => {
                            context.pop();
                        }
                        b"getlastmodified" if context.last() == Some(&Context::Getlastmodified) => {
                            context.pop();
                        }
                        _ => {}
                    }
                }
                Event::GeneralRef(value) => {
                    if let Some(&Context::Href) = context.last() {
                        match value.xml_content() {
                            Ok(entity_content) => {
                                // We need to reconstruct the entity for unescaping.
                                let entity = format!("&{};", entity_content);

                                match unescape(entity.as_str()) {
                                    Ok(unescaped) => {
                                        href_buf.push_str(&unescaped);
                                    }
                                    Err(err) => {
                                        return Err(FSError::MetaFailed(
                                            choose_path(abs_path, &None),
                                            err.into(),
                                        ));
                                    }
                                }
                            }
                            Err(err) => {
                                return Err(FSError::MetaFailed(
                                    choose_path(abs_path, &None),
                                    err.into(),
                                ));
                            }
                        }
                    }
                }
                Event::Text(value) => match context.last() {
                    Some(&Context::Href) => match value.xml_content() {
                        Ok(xml_content) => {
                            href_buf.push_str(&xml_content);
                        }
                        Err(err) => {
                            return Err(FSError::MetaFailed(
                                choose_path(abs_path, &None),
                                err.into(),
                            ));
                        }
                    },
                    Some(&Context::Getcontentlength) => match value.xml_content() {
                        Ok(xml_content) => {
                            if let Ok(parsed) = xml_content.parse::<u64>() {
                                size = Some(parsed);
                            } else {
                                return Err(FSError::MetaFailed(
                                    choose_path(abs_path, &entry_rel_path),
                                    "Empty or invalid content-length".into(),
                                ));
                            }
                        }
                        Err(err) => {
                            return Err(FSError::MetaFailed(
                                choose_path(abs_path, &entry_rel_path),
                                err.into(),
                            ));
                        }
                    },
                    Some(&Context::Creationdate) => match value.xml_content() {
                        Ok(xml_content) => {
                            if let Some(systime) = parse_webdav_datetime(&xml_content) {
                                created = Some(systime);
                            } else {
                                return Err(FSError::MetaFailed(
                                    choose_path(abs_path, &entry_rel_path),
                                    "Empty or invalid creation-date".into(),
                                ));
                            }
                        }
                        Err(err) => {
                            return Err(FSError::MetaFailed(
                                choose_path(abs_path, &entry_rel_path),
                                err.into(),
                            ));
                        }
                    },
                    Some(&Context::Getlastmodified) => match value.xml_content() {
                        Ok(xml_content) => {
                            if let Some(systime) = parse_webdav_datetime(&xml_content) {
                                modified = Some(systime);
                            } else {
                                return Err(FSError::MetaFailed(
                                    choose_path(abs_path, &entry_rel_path),
                                    "Empty or invalid last-modified".into(),
                                ));
                            }
                        }
                        Err(err) => {
                            return Err(FSError::MetaFailed(
                                choose_path(abs_path, &entry_rel_path),
                                err.into(),
                            ));
                        }
                    },
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }

            xml_buf.clear();
        }

        Ok(fs_nodes)
    }

    fn remove(&self, abs_path: &UNPath<Abs>) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match make_url_from_abs(abs_path) {
            Ok(url) => {
                let response = self.start_request(Method::DELETE, &url).send();

                match response {
                    Ok(res) => {
                        if res.status().is_success() {
                            Ok(())
                        } else {
                            match abs_path {
                                UNPath::File(file_path) => Err(FSError::RemoveFileFailed(
                                    file_path.clone(),
                                    "Removal was not successful".into(),
                                )),
                                UNPath::Dir(dir_path) => Err(FSError::RemoveDirFailed(
                                    dir_path.clone(),
                                    "Removal was not successful".into(),
                                )),
                            }
                        }
                    }
                    Err(err) => match abs_path {
                        UNPath::File(file_path) => {
                            Err(FSError::RemoveFileFailed(file_path.clone(), err.into()))
                        }
                        UNPath::Dir(dir_path) => {
                            Err(FSError::RemoveDirFailed(dir_path.clone(), err.into()))
                        }
                    },
                }
            }
            Err(err) => match abs_path {
                UNPath::File(file_path) => {
                    Err(FSError::RemoveFileFailed(file_path.clone(), err.into()))
                }
                UNPath::Dir(dir_path) => {
                    Err(FSError::RemoveDirFailed(dir_path.clone(), err.into()))
                }
            },
        }
    }
}

impl FS for WebDAVFS {
    fn connect(&mut self) -> Result<(), FSError> {
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn block_size(&self) -> FSBlockSize {
        FSBlockSize::new(None, 128 * 1024, None)
    }

    fn meta(&self, abs_path: &UNPath<Abs>) -> Result<FSNodeMetaData, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match make_url_from_abs(abs_path) {
            Ok(url) => {
                let response = self
                    .start_request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
                    .header("Depth", "0")
                    .send()
                    .map_err(|err| FSError::MetaFailed(abs_path.clone(), err.into()))?;

                let xml = response
                    .text()
                    .map_err(|err| FSError::MetaFailed(abs_path.clone(), err.into()))?;

                match self.parse_response(abs_path, true, &xml)?.as_mut_slice() {
                    [fs_node] => {
                        // Type of fs_node.abs_path and abs_path must be the same.
                        if fs_node.abs_path.is_dir() != abs_path.is_dir() {
                            return Err(FSError::MetaFailed(
                                abs_path.clone(),
                                "Path target mismatch".into(),
                            ));
                        }

                        let mut metadata = fs_node.metadata.clone();

                        if !fs_node.abs_path.is_dir()
                            && let Ok(real_size) = self.get_file_size_with_range(abs_path)
                        {
                            metadata.size = real_size;
                        }

                        Ok(metadata)
                    }
                    _ => Err(FSError::MetaFailed(
                        abs_path.clone(),
                        "Response was empty or invalid".into(),
                    )),
                }
            }
            Err(err) => Err(FSError::MetaFailed(abs_path.clone(), err.into())),
        }
    }

    fn list_dir(
        &self,
        abs_dir_path: &NPath<Abs, Dir>,
    ) -> Result<Warned<Vec<FSNode>, String>, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match make_url_from_abs(&abs_dir_path.into()) {
            Ok(url) => {
                let response = self
                    .start_request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
                    .header("Depth", "1")
                    .send()
                    .map_err(|err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()))?;

                let xml = response
                    .text()
                    .map_err(|err| FSError::ListDirFailed(abs_dir_path.clone(), err.into()))?;

                match self.parse_response(&abs_dir_path.into(), false, &xml) {
                    Ok(nodes) => Ok(Warned::new(nodes, vec![])),
                    Err(err) => Err(FSError::ListDirFailed(abs_dir_path.clone(), err.into())),
                }
            }
            Err(err) => Err(FSError::ListDirFailed(abs_dir_path.clone(), err.into())),
        }
    }

    fn remove_file(&self, abs_file_path: &NPath<Abs, File>) -> Result<(), FSError> {
        self.remove(&abs_file_path.into())
    }

    fn remove_dir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<(), FSError> {
        self.remove(&abs_dir_path.into())
    }

    fn mkdir(&self, abs_dir_path: &NPath<Abs, Dir>) -> Result<(), FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match make_url_from_abs(&abs_dir_path.into()) {
            Ok(url) => {
                let response = self
                    .start_request(Method::from_bytes(b"MKCOL").unwrap(), &url)
                    .send();

                match response {
                    Ok(res) => {
                        if res.status().is_success() {
                            Ok(())
                        } else {
                            Err(FSError::MkDirFailed(
                                abs_dir_path.clone(),
                                "mkdir was not successful".into(),
                            ))
                        }
                    }
                    Err(err) => Err(FSError::MkDirFailed(abs_dir_path.clone(), err.into())),
                }
            }
            Err(err) => Err(FSError::MkDirFailed(abs_dir_path.clone(), err.into())),
        }
    }

    fn read_data(&self, abs_file_path: &NPath<Abs, File>) -> Result<Box<dyn Read + Send>, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match make_url_from_abs(&abs_file_path.into()) {
            Ok(url) => {
                let response = self
                    .start_request(Method::GET, &url)
                    .timeout(std::time::Duration::from_secs(self.timeout_secs))
                    .send()
                    .map_err(|err| FSError::ReadFailed(abs_file_path.clone(), err.into()))?;

                let response = response
                    .error_for_status()
                    .map_err(|err| FSError::ReadFailed(abs_file_path.clone(), err.into()))?;

                Ok(Box::new(response))
            }
            Err(err) => Err(FSError::ReadFailed(abs_file_path.clone(), err.into())),
        }
    }

    fn write_data(&self, abs_file_path: &NPath<Abs, File>) -> Result<FSWrite, FSError> {
        if !self.connected {
            return Err(FSError::NotConnected);
        }

        match make_url_from_abs(&abs_file_path.into()) {
            Ok(url) => {
                let client = Arc::new(self.client.clone());
                let username = self.username.clone();
                let password = self.password.clone();
                let timeout_secs = self.timeout_secs;

                let (reader, writer) = pipe()
                    .map_err(|err| FSError::WriteFailed(abs_file_path.clone(), err.into()))?;

                let thread_handle = thread::spawn(move || {
                    let _result = client
                        .request(Method::PUT, url.clone())
                        .timeout(std::time::Duration::from_secs(timeout_secs))
                        .basic_auth(username, Some(password.expose_secret()))
                        .body(reqwest::blocking::Body::new(reader))
                        .send();
                });

                Ok(FSWrite::new(Box::new(writer), Some(thread_handle)))
            }
            Err(err) => Err(FSError::WriteFailed(abs_file_path.clone(), err.into())),
        }
    }
}
