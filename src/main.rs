use std::io::{self, BufReader};
use std::{str, mem};

use quick_xml::Reader;
use quick_xml::events::Event;
use smallstr::SmallString;
use main_error::MainError;

type SmallString64 = SmallString<[u8; 64]>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Release,
    Title,
    Artists,
    Artist,
}

impl Scope {
    fn pop(self) -> Scope {
        match self {
            Scope::Release => Scope::Release,
            Scope::Title => Scope::Release,
            Scope::Artists => Scope::Title,
            Scope::Artist => Scope::Artists,
        }
    }
}


#[derive(Default)]
struct Release {
    id: Option<SmallString64>,
    title: Option<SmallString64>,
    artist: Option<SmallString64>,
}

fn main() -> Result<(), MainError> {
    let xml = BufReader::new(io::stdin());
    let mut reader = Reader::from_reader(xml);
    reader.trim_text(true);

    let mut writer = csv::Writer::from_writer(io::stdout());
    writer.write_record(&["id", "title", "artist"])?;

    let mut count = 0;
    let mut buf = Vec::new();
    let mut scope = Scope::Release;
    let mut release = Release::default();

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) if e.name() == b"release" => {
                scope = Scope::Release;
                release = Release::default();

                if let Some(Ok(attribute)) = e.attributes().find(|a| a.as_ref().map_or(false, |a| a.key == b"id")) {
                    count += 1;
                    if count % 10000 == 0 { eprintln!("{} releases seen", count) }
                    release.id = Some(SmallString::from_str(str::from_utf8(&attribute.value)?));
                }
            },
            Event::End(e) if e.name() == b"release" => {
                // end of release, we must write the csv line if complete
                scope = scope.pop();
                if let Release { id: Some(id), title: Some(title), artist: Some(artist) } = mem::take(&mut release) {
                    writer.write_record(&[id.as_str(), title.as_str(), artist.as_str()])?;
                }
            },

            Event::Start(e) if e.name() == b"title" => scope = Scope::Title,
            Event::End(e) if e.name() == b"title" => scope = scope.pop(),

            Event::Start(e) if e.name() == b"artists" => scope = Scope::Artists,
            Event::End(e) if e.name() == b"artists" => scope = scope.pop(),

            Event::Start(e) if e.name() == b"artist" => scope = Scope::Artist,
            Event::End(e) if e.name() == b"artist" => scope = scope.pop(),

            Event::Text(e) if scope == Scope::Title => {
                let unescaped = e.unescaped()?;
                let text = reader.decode(&unescaped)?;
                release.title = Some(SmallString64::from_str(text));
            },
            Event::Text(e) if scope == Scope::Artist => {
                if release.artist.is_none() {
                    let unescaped = e.unescaped()?;
                    let text = reader.decode(&unescaped)?;
                    release.artist = Some(SmallString64::from_str(text));
                }
            },

            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }

    writer.flush()?;

    Ok(())
}
