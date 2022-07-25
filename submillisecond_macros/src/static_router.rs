use std::path::Path;
use std::{fs, io};

use mime_guess::{mime, Mime};
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

use crate::hquote;
use crate::router::{ItemCatchAll, ItemHandler};

#[derive(Debug)]
pub struct StaticRouter {
    files: Vec<StaticFile>,
    catch_all: Option<ItemHandler>,
}

impl StaticRouter {
    pub fn expand(&self) -> TokenStream {
        let catch_all_expanded = self.expand_catch_all();
        let match_arms = self.expand_match_arms();

        hquote! {
            (|mut req: ::submillisecond::RequestContext| -> ::submillisecond::Response {
                if *req.method() != ::submillisecond::http::Method::GET {
                    return #catch_all_expanded;
                }

                match req.reader.read_to_end() {
                    #match_arms
                    _ => #catch_all_expanded,
                }
            }) as ::submillisecond::Router
        }
    }

    fn expand_catch_all(&self) -> TokenStream {
        ItemCatchAll::expand_catch_all_handler(self.catch_all.as_ref())
    }

    fn expand_match_arms(&self) -> TokenStream {
        let arms = self.files.iter().map(|StaticFile { mime, path, content }| {
            let path = format!("/{path}");
            let mime = mime.to_string();
            let bytes = hquote! { &[#( #content ),*] };

            hquote! {
                #path => {
                    let mut headers = ::submillisecond::http::header::HeaderMap::new();
                    headers.insert(::submillisecond::http::header::CONTENT_TYPE, #mime.parse().unwrap());
                    ::submillisecond::IntoResponse::into_response((headers, #bytes as &'static [u8]))
                }
            }
        });

        hquote! { #( #arms, )* }
    }
}

impl Parse for StaticRouter {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let dir: LitStr = input.parse()?;
        let catch_all = if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            Some(input.parse()?)
        } else {
            None
        };
        let files = walk_dir(dir.value()).map_err(|err| syn::Error::new(dir.span(), err))?;

        Ok(StaticRouter { files, catch_all })
    }
}

#[derive(Debug)]
struct StaticFile {
    mime: Mime,
    path: String,
    content: Vec<u8>,
}

fn walk_dir<P>(base_path: P) -> io::Result<Vec<StaticFile>>
where
    P: AsRef<Path>,
{
    fn walk_nested(base_path: &Path, path: &Path) -> io::Result<Vec<StaticFile>> {
        let dir = fs::read_dir(path)?;
        let mut static_files = Vec::new();
        for entry in dir {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                println!("{:?}", entry.path());
                static_files.extend(walk_nested(base_path, &entry.path())?.into_iter());
            } else {
                let entry_path = entry.path();
                let entry_path = entry_path
                    .strip_prefix(&base_path)
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                let mime = mime_guess::from_path(&entry_path)
                    .first()
                    .unwrap_or(mime::TEXT_PLAIN);

                let content = fs::read(entry.path())?;

                static_files.push(StaticFile {
                    mime,
                    path: entry_path
                        .to_str()
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::Other,
                                "unable to convert path to UTF-8 string",
                            )
                        })?
                        .to_string(),
                    content,
                });
            }
        }

        Ok(static_files)
    }

    walk_nested(base_path.as_ref(), base_path.as_ref())
}
