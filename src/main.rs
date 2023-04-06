use std::convert::TryInto;
use std::io::{self, BufReader};
use std::{mem, str};

use main_error::MainError;
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use smallstr::SmallString;
use smallvec::SmallVec;
use time::Month::January;
use time::{Date, OffsetDateTime};

type SmallString64 = SmallString<[u8; 64]>;
type SmallVec32<T> = SmallVec<[T; 32]>;
type Scope = Vec<Vec<u8>>;

#[derive(Default)]
struct Release {
    id: Option<SmallString64>,
    album: Option<SmallString64>,
    artist: Option<SmallString64>,
    genre: Option<SmallString64>,
    country: Option<SmallString64>,
    released: Option<SmallString64>,
    songs: SmallVec32<(SmallString64, Option<SmallString64>)>, // title, duration
}

fn main() -> Result<(), MainError> {
    let xml = BufReader::new(io::stdin());
    let mut reader = Reader::from_reader(xml);
    let decoder = reader.decoder();
    reader.trim_text(true);

    let mut writer = csv::Writer::from_writer(io::stdout());
    writer.write_record(&[
        "id",
        "title",
        "album",
        "artist",
        "genre",
        "country",
        "released",
        "duration",
        "released-timestamp",
        "duration-float",
    ])?;

    let mut count = 0;
    let mut buf = Vec::new();
    let mut scope = Scope::default();
    let mut release = Release::default();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                scope.push(e.name().into_inner().to_owned());

                match e.name() {
                    QName(b"release") => {
                        release = Release::default();
                        if let Some(Ok(attribute)) = e
                            .attributes()
                            .find(|a| a.as_ref().map_or(false, |a| a.key == QName(b"id")))
                        {
                            release.id =
                                Some(SmallString::from_str(str::from_utf8(&attribute.value)?));
                        }
                    }
                    _ => (),
                }
            }
            Event::End(e) => {
                // only pop if we entered in a scope
                if scope.last().as_ref().map(AsRef::as_ref) == Some(e.name().into_inner()) {
                    scope.pop();
                }

                match e.name() {
                    QName(b"release") => {
                        // end of release, we must write the csv line if complete
                        if let Release {
                            id: Some(id),
                            album: Some(album),
                            artist: Some(artist),
                            genre,
                            country,
                            released,
                            songs,
                        } = mem::take(&mut release)
                        {
                            let id: usize = id.parse()?;

                            for (i, (title, duration)) in songs.into_iter().enumerate().take(100) {
                                let id = id * 100 + i;
                                let id = id.to_string();

                                let duration_float = duration.as_ref().and_then(|d| {
                                    d.split_once(':').map(|(m, s)| format!("{}.{}", m, s))
                                });

                                let released_timestamp = released.as_ref().and_then(|d| {
                                    d.split_once('-').map(|(year, tail)| {
                                        let year = year.parse().unwrap();
                                        let result = match tail.split_once('-') {
                                            Some((month, day)) => {
                                                // month
                                                let month = month.parse::<u8>().unwrap_or_default();
                                                let month = month.try_into().unwrap_or(January);
                                                // day
                                                let day = day.parse::<u8>().unwrap_or_default();
                                                let day = day.clamp(1, 27);
                                                // the whole date
                                                Date::from_calendar_date(year, month, day)
                                            }
                                            None => Date::from_calendar_date(year, January, 1),
                                        };

                                        let date = result.unwrap();
                                        OffsetDateTime::from_unix_timestamp(0)
                                            .unwrap()
                                            .replace_date(date)
                                            .unix_timestamp()
                                            .to_string()
                                    })
                                });

                                writer.write_record(&[
                                    id.as_str(),
                                    title.as_str(),
                                    album.as_str(),
                                    artist.as_str(),
                                    genre.as_deref().unwrap_or_default(),
                                    country.as_deref().unwrap_or_default(),
                                    released.as_deref().unwrap_or_default(),
                                    duration.as_deref().unwrap_or_default(),
                                    released_timestamp.as_deref().unwrap_or_default(),
                                    duration_float.as_deref().unwrap_or_default(),
                                ])?;
                            }
                        }
                    }
                    _ => (),
                }
            }

            Event::Text(e) => {
                let unescaped = e.unescape()?;
                let text = decoder.decode(unescaped.as_bytes())?;

                if scope == [&b"releases"[..], b"release", b"title"] {
                    release.album = Some(SmallString64::from_str(&text));
                }

                if scope == [&b"releases"[..], b"release", b"genres", b"genre"] {
                    release.genre = Some(SmallString64::from_str(&text));
                }

                if scope == [&b"releases"[..], b"release", b"country"] {
                    release.country = Some(SmallString64::from_str(&text));
                }

                if scope == [&b"releases"[..], b"release", b"released"] {
                    release.released = Some(SmallString64::from_str(&text));
                }

                if scope == [&b"releases"[..], b"release", b"artists", b"artist", b"name"] {
                    release.artist = Some(SmallString64::from_str(&text));
                }

                if scope == [&b"releases"[..], b"release", b"tracklist", b"track", b"title"] {
                    count += 1;
                    if count % 10000 == 0 {
                        eprintln!("{} songs seen", count)
                    }
                    release.songs.push((SmallString64::from_str(&text), None));
                }

                if scope == [&b"releases"[..], b"release", b"tracklist", b"track", b"duration"] {
                    if let Some((_title, duration)) = release.songs.last_mut() {
                        *duration = Some(SmallString64::from_str(&text));
                    }
                }
            }

            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }

    eprintln!("{} songs seen", count);

    writer.flush()?;

    Ok(())
}
