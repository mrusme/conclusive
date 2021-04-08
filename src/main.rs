use std::io;
use std::env;
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

pub struct App<'a> {
  pub stats: Vec<(&'a str, u64)>
}

impl<'a> App<'a> {
  pub fn new(stats: Vec<(&'a str, u64)>) -> App<'a> {
    App {
      stats: stats,
    }
  }
}

fn main() -> Result<(), io::Error> {
  let stdout = io::stdout().into_raw_mode()?;
  let backend = TermionBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let plausible_token = env::var("PLAUSIBLE_TOKEN").unwrap();

  let client = reqwest::blocking::Client::new();
  let response = client.get(format!("https://plausible.io/api/v1/stats/timeseries?site_id={site_id}&period={period}", site_id = "xn--gckvb8fzb.com", period = "7d"))
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



  let app = App::new(stats);

  terminal.clear()?;
  terminal.draw(|f| {
    let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(1)
    .constraints(
      [
      Constraint::Percentage(10),
      Constraint::Percentage(40),
      Constraint::Percentage(50)
      ].as_ref()
      )
    .split(f.size());

    let block_overview = Block::default()
    .title("Overview")
    .borders(Borders::ALL);
    f.render_widget(block_overview, chunks[0]);

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
    f.render_widget(barchart, chunks[1]);

    let chunks2 = Layout::default()
    .direction(Direction::Horizontal)
    .margin(0)
    .constraints(
      [
      Constraint::Percentage(50),
      Constraint::Percentage(50)
      ].as_ref()
      )
    .split(chunks[2]);

    let block_top_sources = Block::default()
    .title("Top Sauces")
    .borders(Borders::ALL);
    f.render_widget(block_top_sources, chunks2[0]);

    let block_top_pages = Block::default()
    .title("Top Pages")
    .borders(Borders::ALL);
    f.render_widget(block_top_pages, chunks2[1]);
  })
}

