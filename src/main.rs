use std::io;
use std::env;
extern crate clap;
use clap::{
  Arg,
  App
};
use termion::raw::IntoRawMode;
use tui::{
  Terminal,
  backend::TermionBackend,
  widgets::{
    Block,
    Borders
  },
  layout::{
    Layout,
    Constraint,
    Direction
  },
  symbols,
  style::{
    Color,
    Style
  },
  widgets::{
    BarChart,
  },
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct TimeseriesResult {
    date: String,
    visitors: u64,
}

#[derive(Deserialize, Debug)]
struct TimeseriesResponse {
    results: Vec<TimeseriesResult>,
}

pub struct TUI<'a> {
  pub stats: Vec<(&'a str, u64)>
}

impl<'a> TUI<'a> {
  pub fn new(stats: Vec<(&'a str, u64)>) -> TUI<'a> {
    TUI {
      stats: stats,
    }
  }
}

fn main() -> Result<(), io::Error> {
  let args = App::new("conclusive")
    .version("0.1.0")
    .about("A command line client for Plausible Analytics.")
    .author("マリウス <marius@マリウス.com>")
    .arg(Arg::with_name("SITE-ID")
      .help("Site ID")
      .required(true)
      .index(1)
      .takes_value(true))
    .arg(Arg::with_name("period")
      .help("Period")
      .short("p")
      .long("period")
      .takes_value(true))
    .get_matches();

  let site_id = args.value_of("SITE-ID").unwrap();
  let period = args.value_of("period").unwrap_or("7d");

  let plausible_token = env::var("PLAUSIBLE_TOKEN").unwrap();

  let client = reqwest::blocking::Client::new();
  let response = client.get(format!("https://plausible.io/api/v1/stats/timeseries?site_id={site_id}&period={period}", site_id = site_id, period = period))
    .bearer_auth(plausible_token)
    .send();
  let resp = response.unwrap();

  if resp.status().is_success() == false {
    println!("Error: {:#?}", resp);
    std::process::exit(1);
  }

  let timeseries: TimeseriesResponse = resp.json().unwrap();

  let mut stats: Vec<(&str, u64)> = Vec::new();

  for result in timeseries.results.iter() {
    let len = result.date.len();
    stats.push((&result.date[len-2..], result.visitors));
  }

  println!("{:#?}", stats);

  let stdout = io::stdout().into_raw_mode()?;
  let backend = TermionBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let app = TUI::new(stats);

  terminal.clear()?;
  terminal.draw(|f| {
    let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(1)
    .constraints(
      [
      Constraint::Percentage(100)
      ].as_ref()
      )
    .split(f.size());

    let barchart = BarChart::default()
    .block(Block::default().borders(Borders::ALL).title("Stats"))
    .data(&app.stats)
    .bar_width(3)
    .bar_gap(2)
    .bar_set(symbols::bar::NINE_LEVELS)
    .value_style(
      Style::default()
      .fg(Color::Black)
      .bg(Color::Green),
      )
    .label_style(Style::default().fg(Color::Yellow))
    .bar_style(Style::default().fg(Color::Green));
    f.render_widget(barchart, chunks[0]);
  })
}

