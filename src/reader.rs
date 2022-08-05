#[derive(Clone, Debug, Default)]
pub struct UriReader {
    pub uri: String,
    pub cursor: usize,
}

impl UriReader {
    pub fn new(uri: String) -> UriReader {
        UriReader { uri, cursor: 0 }
    }

    pub fn peek(&self, len: usize) -> &str {
        let read_attempt = self.cursor + len;
        if self.uri.len() >= read_attempt {
            return &self.uri[self.cursor..read_attempt];
        }
        ""
    }

    pub fn is_dangling_slash(&self) -> bool {
        self.uri.len() == self.cursor || &self.uri[self.cursor..self.cursor + 1] == "/"
    }

    pub fn read(&mut self, len: usize) {
        self.cursor += len;
    }

    pub fn read_matching(&mut self, s: &str) -> bool {
        let read_to = self.cursor + s.len();
        if read_to > self.uri.len() {
            return false;
        }

        if &self.uri[self.cursor..read_to] == s {
            self.cursor = read_to;
            return true;
        }

        false
    }

    pub fn ensure_next_slash(&mut self) -> bool {
        self.read_matching("/")
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    pub fn is_empty(&self, allow_trailing_slash: bool) -> bool {
        if allow_trailing_slash {
            self.uri.len() <= self.cursor || &self.uri[self.cursor..] == "/"
        } else {
            self.uri.len() <= self.cursor
        }
    }

    pub fn read_param(&mut self) -> Option<&str> {
        let initial_cursor = self.cursor;
        while !self.is_empty(false) {
            if self.peek(1) != "/" {
                self.read(1);
            } else {
                break;
            }
        }
        // if nothing was found, return none
        if initial_cursor == self.cursor {
            return None;
        }
        // read the param
        Some(&self.uri[initial_cursor..self.cursor])
    }

    pub fn ends_with(&self, suffix: &str) -> bool {
        if self.cursor >= self.uri.len() {
            return false;
        }
        let end = &self.uri[self.cursor..];
        end == suffix
    }

    pub fn read_to_end(&self) -> &str {
        &self.uri[self.cursor..]
    }
}

#[cfg(test)]
mod tests {
    use super::UriReader;

    #[test]
    fn peek_empty_string() {
        let reader = UriReader::new("".to_string());
        assert_eq!(reader.peek(5), "");
    }

    #[test]
    fn peek_path() {
        let mut reader = UriReader::new("/alive".to_string());
        assert_eq!(reader.peek(3), "/al");
        reader.read(3);
        assert_eq!(reader.peek(3), "ive");
        reader.read(3);
        assert_eq!(reader.peek(3), "");
        reader.read(3);
    }
}
