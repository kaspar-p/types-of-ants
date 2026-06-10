use ant_library::routes::Routes;
use anyhow::Context;
use axum::{Json, http::StatusCode, response::IntoResponse, routing::post};
use chrono::{DateTime, Utc};
use escpos::{
    driver::{Driver, UsbDriver},
    printer::Printer,
    utils::{DebugMode, Protocol, RealTimeStatusRequest, RealTimeStatusResponse},
};
use serde::{Deserialize, Serialize};
use strum::EnumString;
use tracing::info;

use crate::{err::AntPrintingPressError, state::AntPrintingPressState};

const LINE_WIDTH: usize = 32;

fn get_printer() -> Result<Printer<UsbDriver>, anyhow::Error> {
    const VENDOR_ID: u16 = 0x0483;
    const PRODUCT_ID: u16 = 0x070b;
    let driver = UsbDriver::open(VENDOR_ID, PRODUCT_ID, None, None).context("driver init")?;
    let mut printer = Printer::new(driver.clone(), Protocol::default(), None);

    printer.init().context("printer init")?;
    printer
        .debug_mode(Some(DebugMode::Dec))
        .real_time_status(RealTimeStatusRequest::Printer)
        .context("printer")?
        .real_time_status(RealTimeStatusRequest::RollPaperSensor)
        .context("roll")?
        .send_status()?;

    let mut buf = [0; 32];
    driver.read(&mut buf).context("read")?;

    let status = RealTimeStatusResponse::parse(RealTimeStatusRequest::Printer, buf[0])
        .context("debug status")?;
    info!("initial printer status: {status:?}");

    return Ok(printer);
}

#[derive(Serialize, Deserialize, EnumString, strum::Display)]
pub enum SecretEncoding {
    #[strum(serialize = "string")]
    String,
    #[strum(serialize = "base64")]
    Base64,
}

impl Default for SecretEncoding {
    fn default() -> Self {
        Self::String
    }
}

#[derive(Serialize, Deserialize)]
pub struct PrintSecretRequest {
    pub id: String,

    #[serde(default = "default_now")]
    pub created: DateTime<Utc>,

    #[serde(default)]
    pub encoding: SecretEncoding,

    /// Base64 encoded if binary!
    pub content: String,
}

fn default_now() -> DateTime<Utc> {
    Utc::now()
}

async fn print_secret(
    Json(req): Json<PrintSecretRequest>,
) -> Result<impl IntoResponse, AntPrintingPressError> {
    let id_prefix = "id:   ";
    let type_prefix = "type: ";

    let max_len = LINE_WIDTH - id_prefix.len();
    if req.id.len() > max_len {
        return Err(AntPrintingPressError::ValidationMessage(format!(
            "Secret [{}] is too long, must be < {max_len} chars",
            req.id
        )));
    }

    let mut printer = get_printer()?;

    printer
        .feed()?
        .bold(true)?
        .writeln(&"=".repeat(LINE_WIDTH))?
        .justify(escpos::utils::JustifyMode::CENTER)?
        .writeln("++ typesofants.org secret ++")?
        .justify(escpos::utils::JustifyMode::LEFT)?
        .bold(false)?
        .writeln(&format!("{id_prefix}{}", req.id))?
        .writeln(&format!("{type_prefix}{}", req.encoding.to_string()))?
        .feed()?
        .writeln(&req.content)?
        .bold(true)?
        .writeln(&"=".repeat(LINE_WIDTH))?
        .feeds(2)?
        .print_cut()?;

    return Ok(StatusCode::OK);
}

async fn print_wrapped_message(body: String) -> Result<impl IntoResponse, AntPrintingPressError> {
    let mut printer = get_printer()?;

    printer
        .feed()?
        .writeln(&"=".repeat(LINE_WIDTH))?
        .writeln(&body)?
        .writeln(&"=".repeat(LINE_WIDTH))?
        .feeds(2)?
        .print_cut()?;

    return Ok(StatusCode::OK);
}

async fn print_raw_message(body: String) -> Result<impl IntoResponse, AntPrintingPressError> {
    let mut printer = get_printer()?;

    printer.writeln(&body)?.print_cut()?;

    return Ok(StatusCode::OK);
}

pub fn routes() -> Routes<AntPrintingPressState> {
    Routes::new()
        .post("/secret", post(print_secret))
        .post("/msg", post(print_wrapped_message))
        .post("/raw-msg", post(print_raw_message))
}
