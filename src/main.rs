use std::io::{self, BufReader};
use std::{str, mem};

use quick_xml::Reader;
use quick_xml::events::Event;
use smallstr::SmallString;
use smallvec::SmallVec;
use main_error::MainError;

type SmallString64 = SmallString<[u8; 64]>;
type SmallVec32<T> = SmallVec<[T; 32]>;
type Scope = Vec<Vec<u8>>;

#[derive(Default)]
struct Release {
    id: Option<SmallString64>,
    album: Option<SmallString64>,
    artist: Option<SmallString64>,
    songs: SmallVec32<SmallString64>,
}

fn main() -> Result<(), MainError> {
    let xml = BufReader::new(io::stdin());
    let mut reader = Reader::from_reader(xml);
    reader.trim_text(true);

    let mut writer = csv::Writer::from_writer(io::stdout());
    writer.write_record(&["id", "title", "album", "artist"])?;

    let mut count = 0;
    let mut buf = Vec::new();
    let mut scope = Scope::default();
    let mut release = Release::default();

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) => {
                scope.push(e.name().to_owned());

                match e.name() {
                    b"release" => {
                        release = Release::default();
                        if let Some(Ok(attribute)) = e.attributes().find(|a| a.as_ref().map_or(false, |a| a.key == b"id")) {
                            release.id = Some(SmallString::from_str(str::from_utf8(&attribute.value)?));
                        }
                    }
                    _ => (),
                }
            },
            Event::End(e) => {
                // only pop if we entered in a scope
                if scope.last().as_ref().map(AsRef::as_ref) == Some(e.name()) {
                    scope.pop();
                }

                match e.name() {
                    b"release" => {
                        // end of release, we must write the csv line if complete
                        if let Release { id: Some(id), album: Some(album), artist: Some(artist), songs } = mem::take(&mut release) {
                            for title in songs {
                                writer.write_record(&[
                                    id.as_str(),
                                    title.as_str(),
                                    album.as_str(),
                                    artist.as_str()])?;
                            }
                        }
                    },
                    _ => (),
                }
            },

            Event::Text(e) => {
                let unescaped = e.unescaped()?;
                let text = reader.decode(&unescaped)?;

                if scope == [&b"releases"[..], b"release", b"title"] {
                    release.album = Some(SmallString64::from_str(text));
                }

                if scope == [&b"releases"[..], b"release", b"artists", b"artist", b"name"] {
                    release.artist = Some(SmallString64::from_str(text));
                }

                if scope == [&b"releases"[..], b"release", b"tracklist", b"track", b"title"] {
                    count += 1;
                    if count % 10000 == 0 { eprintln!("{} songs seen", count) }
                    release.songs.push(SmallString64::from_str(text));
                }
            },

            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }

    eprintln!("{} songs seen", count);

    writer.flush()?;

    Ok(())
}
