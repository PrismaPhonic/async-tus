use std::io::Read;
pub use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use url::Url;
use crate::Error;

/// Version of the TUS protocol we're configured to use.
pub const TUS_VERSION: &'static str = "1.0.0";

/// Default is 4 meg chunks

const TUS_RESUMABLE: &'static str = "tus-resumable";
const UPLOAD_OFFSET: &'static str = "upload-offset";
const UPLOAD_LENGTH: &'static str = "upload-length";
const OFFSET_OCTET_STREAM: &'static str = "application/offset+octet-stream";

/// Returns the minimum set of headers required to make a TUS request. This should be used as the
/// basis for constructing your headers.
pub fn default_headers(size: u64) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(TUS_RESUMABLE, HeaderValue::from_static(TUS_VERSION));
    headers.insert(UPLOAD_LENGTH, HeaderValue::from_str(&format!("{}", size)).unwrap());
    headers.insert(reqwest::header::CONTENT_TYPE, HeaderValue::from_static(OFFSET_OCTET_STREAM));
    headers
}

/// A client for a TUS endpoint. This leaks a lot of the implementation details of reqwest.
pub struct Client {
    url: Url,
    headers: HeaderMap,
    // TODO(richo) Make this generic over some trait so we can test it
    client: reqwest::blocking::Client,
}

impl Client {
    /// Creates a new Client.
    ///
    /// Headers should be a HeaderMap preloaded with all necessary information to communicate with
    /// the endpoint, including eg authentication information.
    pub fn new(url: Url, headers: HeaderMap) -> Client {
        Client {
            url,
            headers,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Uploads all content from `reader` to the endpoint, consuming this Client.
    pub fn upload<T>(self, reader: T) -> Result<usize, Error>
        where T: Read {
        self.upload_inner(reader, |chunk, offset| {
            let mut headers = self.headers.clone();
            headers.insert(UPLOAD_OFFSET.clone(), HeaderValue::from_str(&format!("{}", offset))?);
            headers.remove("upload-length");
            self.upload_chunk(chunk, headers)
        })
    }

    fn upload_inner<T, U>(&self, mut reader: T, mut cb: U) -> Result<usize, Error>
        where T: Read,
              U: FnMut(Vec<u8>, usize) -> Result<usize, Error>,
    {
        let mut offset = 0;
        loop {
            let mut chunk = vec![0; CHUNK_SIZE];
            let bytes_read = reader.read(&mut chunk)
                .map_err(|e| Error::IoError(e))?;
            chunk.truncate(bytes_read);
            if bytes_read == 0 {
                return Ok(offset)
            }
            chunk.truncate(bytes_read);
            cb(chunk, offset)?;
            offset += bytes_read;
        }
    }

    fn upload_chunk(&self, chunk: Vec<u8>, headers: HeaderMap) -> Result<usize, Error> {
        let len = chunk.len();
        let res = self.client
            .patch(self.url.as_str())
            .body(chunk)
            .headers(headers)
            .send()
            .map_err(|e| Error::ReqwestError(e))?;

        if res.status() != reqwest::StatusCode::NO_CONTENT {
            return Err(Error::StringError(format!("Did not save chunk: {} -> {}", res.status(), &res.text()?)))
        }
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{Seek, Write};
    use self::rand::{thread_rng, Rng};

    extern crate rand;
    extern crate tempfile;

    /// Create a dummy client for use in tests
    fn test_client() -> Client {
        Client {
            url: "https://test-url.com/foo".parse().expect("Couldn't parse URL"),
            headers: HeaderMap::new(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Creates a file filled with entropy, and returns it along with the entropy used.
    ///
    /// The buffer is intentionally not a very even size, to ensure that tests that verify chunking
    /// at 1k boundaries do the right thing.
    fn entropy_filled_file<'a>() -> (File, Vec<u8>) {
        use tests::rand::prelude::*;
        let mut tmp = tempfile::tempfile().expect("Couldn't create tempfile");
        let mut rng = thread_rng();

        let mut bytes = vec![0; 1024 * 1024 * 12 + 768];
        rng.fill(&mut bytes[..]);
        let written = tmp.write(&mut bytes).expect("Couldn't fill buffer");
        assert!(written > 0, "Didn't write anything to the tempfile");
        tmp.seek(std::io::SeekFrom::Start(0)).expect("Couldn't seek");

        (tmp, bytes)
    }

    #[test]
    fn test_chunking_works() {
        let (file, bytes) = entropy_filled_file();
        let client = test_client();

        let mut vec: Vec<u8> = vec![];
        client.upload_inner(file, |chunk, _| {
            vec.extend(&chunk);
            Ok(chunk.len())
        }).expect("Didn't upload_inner");

        assert_eq!(&vec[..], &bytes[..]);
    }

    #[test]
    fn test_entropy_works() {
        let (mut file, bytes) = entropy_filled_file();
        let mut vec = Vec::with_capacity(1024);
        file.read_to_end(&mut vec).expect("Couldn't fill buffer");
        assert_eq!(&bytes[..], &vec[..]);
    }

    #[test]
    #[ignore]
    fn test_uploads_a_file() {
        let file = File::open("/tmp/test.mp4").expect("Couldn't open file");
        let size = file.metadata().expect("Couldn't get metadata").len();

        // Get an upload link
        let headers = default_headers(size);
        let resp = reqwest::blocking::Client::new()
            .post("https://master.tus.io/files/")
            .headers(headers)
            .send()
            .expect("couldn't get upload location");

        let loc = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .expect("didn't get a location header")
            .to_str()
            .expect("couldn't parse location header")
            .parse()
            .expect("Couldn't parse location header into a url");

        let headers = default_headers(size);
        let client = Client::new(loc, headers);
        client.upload(file).unwrap();
    }
}
