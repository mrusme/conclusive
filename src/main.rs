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
    Borders,
    Cell,
    Row,
    Table
  },
  layout::{
    Alignment,
    Layout,
    Constraint,
    Direction
  },
  symbols,
  style::{
    Color,
    Style
  },
  text::{
    Spans
  },
  widgets::{
    BarChart,
    Paragraph,
  },
};
use serde::Deserialize;
use serde::de;

const API_BASE_URL: &'static str = "https://plausible.io/api/v1/stats";

#[derive(Deserialize, Debug)]
struct TopPageResult {
    bounce_rate: Option<f32>,
    page: String,
    visitors: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct TopSourceResult {
    bounce_rate: Option<f32>,
    source: String,
    visitors: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct AggregateValue {
    value: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct AggregateResult {
    bounce_rate: AggregateValue,
    pageviews: AggregateValue,
    visit_duration: AggregateValue,
    visitors: AggregateValue,
}

#[derive(Deserialize, Debug)]
struct TimeseriesResult {
    date: String,
    visitors: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct ApiResponse<T> {
    results: T,
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

fn req<T: de::DeserializeOwned>(endpoint: &str, token: &str)
-> Result<ApiResponse<T>, reqwest::blocking::Response> {
  let client = reqwest::blocking::Client::new();
  let response = client.get(endpoint)
    .bearer_auth(token)
    .send();
  let resp = response.unwrap();

  if resp.status().is_success() == false {
    return Err(resp);
  }

  let timeseries: ApiResponse<T> = resp.json().unwrap();
  return Ok(timeseries);
}

fn main()
-> Result<(), io::Error> {
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
      .short('p')
      .long("period")
      .takes_value(true))
    .get_matches();

  let site_id = args.value_of("SITE-ID").unwrap();
  let period = args.value_of("period").unwrap_or("30d");

  let plausible_token = env::var("PLAUSIBLE_TOKEN").unwrap();

  let aggregate: ApiResponse<AggregateResult> =
    match req(&format!(
      "{api}/aggregate?site_id={site_id}&period={period}&metrics=visitors,pageviews,bounce_rate,visit_duration",
      api = API_BASE_URL,
      site_id = site_id,
      period = period
    ), &plausible_token) {
    Err(e) => {
      println!("Error: {:#?}", e);
      std::process::exit(1);
    },
    Ok(r) => r,
  };

  let timeseries: ApiResponse<Vec<TimeseriesResult>> =
    match req(&format!(
      "{api}/timeseries?site_id={site_id}&period={period}",
      api = API_BASE_URL,
      site_id = site_id,
      period = period
    ), &plausible_token) {
    Err(e) => {
      println!("Error: {:#?}", e);
      std::process::exit(1);
    },
    Ok(r) => r,
  };

  let top_sources: ApiResponse<Vec<TopSourceResult>> =
    match req(&format!(
      "{api}/breakdown?site_id={site_id}&period={period}&{args}",
      api = API_BASE_URL,
      site_id = site_id,
      period = period,
      args = "property=visit:source&metrics=visitors,bounce_rate&limit=10"
    ), &plausible_token) {
    Err(e) => {
      println!("Error: {:#?}", e);
      std::process::exit(1);
    },
    Ok(r) => r,
  };

  let top_pages: ApiResponse<Vec<TopPageResult>> =
    match req(&format!(
      "{api}/breakdown?site_id={site_id}&period={period}&{args}",
      api = API_BASE_URL,
      site_id = site_id,
      period = period,
      args = "property=event:page&metrics=visitors,bounce_rate&limit=10"
    ), &plausible_token) {
    Err(e) => {
      println!("Error: {:#?}", e);
      std::process::exit(1);
    },
    Ok(r) => r,
  };

  let mut stats: Vec<(&str, u64)> = Vec::new();

  for result in timeseries.results.iter() {
    let len = result.date.len();
    let visitors: u64 = result.visitors.unwrap_or(0);
    stats.push((&result.date[len-2..], visitors));
  }

  println!("{:#?}", stats);

  let stdout = io::stdout().into_raw_mode()?;
  let backend = TermionBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let app = TUI::new(stats);

  terminal.clear()?;
  let _drawn = match terminal.draw(|f| {
    let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(1)
    .constraints(
      [
      Constraint::Length(5),
      Constraint::Min(10),
      Constraint::Length(16)
      ].as_ref()
      )
    .split(f.size());

    let layout_overview = Layout::default()
    .direction(Direction::Horizontal)
    .margin(0)
    .constraints(
      [
      Constraint::Percentage(25),
      Constraint::Percentage(25),
      Constraint::Percentage(25),
      Constraint::Percentage(25)
      ].as_ref()
      )
    .split(chunks[0]);

    // Total Visitors
    let block_overview_visitors = Block::default()
      .title("Total Visitors")
      .borders(Borders::ALL);

    let overview_visitors_text = vec![
      Spans::from(""),
      Spans::from(format!("{total_visitors}", total_visitors = aggregate.results.visitors.value.unwrap_or(0)))
    ];

    let overview_visitors = Paragraph::new(overview_visitors_text)
      .style(Style::default())
      .block(block_overview_visitors)
      .alignment(Alignment::Center);

    f.render_widget(overview_visitors, layout_overview[0]);

    // Total Pageviews
    let block_overview_pageviews = Block::default()
      .title("Total Pageviews")
      .borders(Borders::ALL);

    let overview_pageviews_text = vec![
      Spans::from(""),
      Spans::from(format!("{total_pageviews}", total_pageviews = aggregate.results.pageviews.value.unwrap_or(0)))
    ];

    let overview_pageviews = Paragraph::new(overview_pageviews_text)
      .style(Style::default())
      .block(block_overview_pageviews)
      .alignment(Alignment::Center);

    f.render_widget(overview_pageviews, layout_overview[1]);

    // Bounce Rate
    let block_overview_bounce = Block::default()
      .title("Bounce Rate")
      .borders(Borders::ALL);

    let overview_bounce_text = vec![
      Spans::from(""),
      Spans::from(format!("{bounce_rate}%", bounce_rate = aggregate.results.bounce_rate.value.unwrap_or(0)))
    ];

    let overview_bounce = Paragraph::new(overview_bounce_text)
      .style(Style::default())
      .block(block_overview_bounce)
      .alignment(Alignment::Center);

    f.render_widget(overview_bounce, layout_overview[2]);

    // Visit Duration
    let block_overview_duration = Block::default()
      .title("Visit Duration")
      .borders(Borders::ALL);

    let overview_duration_text = vec![
      Spans::from(""),
      Spans::from(format!("{visit_duration}s", visit_duration = aggregate.results.visit_duration.value.unwrap_or(0)))
    ];

    let overview_duration = Paragraph::new(overview_duration_text)
      .style(Style::default())
      .block(block_overview_duration)
      .alignment(Alignment::Center);

    f.render_widget(overview_duration, layout_overview[3]);

    // Bar Chart
    let barchart = BarChart::default()
    .block(Block::default().borders(Borders::ALL).title("Stats"))
    .data(&app.stats)
    .bar_width(3)
    .bar_gap(2)
    .bar_set(symbols::bar::NINE_LEVELS)
    .value_style(
      Style::default()
      .fg(Color::Black)
      .bg(Color::White),
      )
    .label_style(Style::default().fg(Color::White))
    .bar_style(Style::default().fg(Color::Rgb(107, 104, 242)));
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


    let normal_style = Style::default().bg(Color::White);

    let header_cells = ["Visitors", "Source", "BNC"]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Black)));
    let header = Row::new(header_cells)
    .style(normal_style)
    .height(1)
    .bottom_margin(1);

    let rows = top_sources.results.iter().map(|item| {
      let cells = vec![
        Cell::from(format!("{}", item.visitors.unwrap_or(0))),
        Cell::from(format!("{}", item.source)),
        Cell::from(format!("{}%", item.bounce_rate.unwrap_or(0.0)))
      ];
      Row::new(cells).height(1 as u16).bottom_margin(1)
    });

    let t = Table::new(rows)
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Top Sauces"))
    .widths(&[
      Constraint::Length(10),
      Constraint::Min(16),
      Constraint::Length(5),
      ]);
    f.render_widget(t, chunks2[0]);


    let header2_cells = ["Visitors", "Page", "BNC"]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Black)));
    let header2 = Row::new(header2_cells)
    .style(normal_style)
    .height(1)
    .bottom_margin(1);
    let rows2 = top_pages.results.iter().map(|item| {
      let cells2 = vec![
        Cell::from(format!("{}", item.visitors.unwrap_or(0))),
        Cell::from(format!("{}", item.page)),
        Cell::from(format!("{}%", item.bounce_rate.unwrap_or(0.0)))
      ];
      Row::new(cells2).height(1 as u16).bottom_margin(1)
    });
    let t2 = Table::new(rows2)
    .header(header2)
    .block(Block::default().borders(Borders::ALL).title("Top Pages"))
    .widths(&[
      Constraint::Length(10),
      Constraint::Min(16),
      Constraint::Length(5),
      ]);
    f.render_widget(t2, chunks2[1]);
  }) {
    Ok(_) => return Ok(()),
    Err(e) => return Err(e),
  };
}

