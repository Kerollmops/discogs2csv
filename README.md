# Discogs2csv

An little tool that converts [a Discogs release XML dump](https://data.discogs.com/) into a CSV.

## Installation

```bash
cargo install discogs2csv
```

## Usage

First download a release dump from the Discogs website:

```bash
curl -O 'https://discogs-data-dumps.s3-us-west-2.amazonaws.com/data/2023/discogs_20230301_releases.xml.gz'
```

Then simply feed it to the `discogs2csv` command:

```bash
gunzip --stdout discogs_20230301_releases.xml.gz | discogs2csv > tracks.csv
```

Optionally you could convert this CSV into a typed JSON-line:

```bash
cargo install csv2ndjson-lite
cat tracks.csv | csv2ndjson-lite --arrays genre --numbers id released-timestamp duration-float > tracks.ndjson
```
