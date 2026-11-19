use std::borrow::Cow;
use std::{fmt, marker};

use jiff::ZonedDateTime;
use jiff::civil::Date;
use jiff::fmt::temporal::Pieces;
use jiff::tz::TimeZone;
use serde::de::Error as _;
use serde::{Deserialize, Serialize};

pub const GAS_USAGE_ENTRIES_PER_PAGE: usize = 32;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum RawCommandArgument {
	Float(f64),
	String(String),
}

impl From<f64> for RawCommandArgument {
	fn from(val: f64) -> Self {
		RawCommandArgument::Float(val)
	}
}

impl From<String> for RawCommandArgument {
	fn from(val: String) -> Self {
		RawCommandArgument::String(val)
	}
}

impl<'s> From<&'s str> for RawCommandArgument {
	fn from(val: &'s str) -> Self {
		RawCommandArgument::String(val.to_owned())
	}
}

impl fmt::Display for RawCommandArgument {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			RawCommandArgument::Float(v) => write!(f, "{v}"),
			RawCommandArgument::String(ref v) => write!(f, "{v}"),
		}
	}
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum RawCommand {
	Ping,
	Disconnect,
	Get(Cow<'static, str>),
	Put(Cow<'static, str>, RawCommandArgument),
}

impl RawCommand {
	pub fn get_reply_stanza_filters(&self) -> (Option<&'static str>, Option<&'static str>, Option<&'static str>) {
		match *self {
			RawCommand::Ping => (None, Some("presence"), None),
			RawCommand::Get(..) | RawCommand::Put(..) => (None, None, Some("chat")),
			_ => (None, None, None),
		}
	}
}

impl fmt::Display for RawCommand {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			RawCommand::Ping => f.write_str("Ping"),
			RawCommand::Disconnect => f.write_str("Disconnect"),
			RawCommand::Get(ref url) => write!(f, "Get, url: {url}"),
			RawCommand::Put(ref url, ref val) => write!(f, "Put, url: {url}, value: {val}"),
		}
	}
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum RawCommandResult {
	Empty,
	Json(String),
}

pub struct Command<RES>(RawCommand, marker::PhantomData<RES>);

impl<RES: serde::de::DeserializeOwned> From<Command<RES>> for RawCommand {
	fn from(s: Command<RES>) -> Self {
		s.0
	}
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatValue {
	pub id: String,
	#[serde(rename = "type")]
	pub kind: String,
	#[serde(deserialize_with = "u8_as_bool")]
	pub recordable: bool,
	#[serde(deserialize_with = "u8_as_bool")]
	pub writeable: bool,
	pub value: f64,
	pub unit_of_measure: String,
	pub min_value: f64,
	pub max_value: f64,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutdoorTempValue {
	pub id: String,
	#[serde(rename = "type")]
	pub kind: String,
	#[serde(deserialize_with = "u8_as_bool")]
	pub recordable: bool,
	#[serde(deserialize_with = "u8_as_bool")]
	pub writeable: bool,
	pub value: f64,
	pub unit_of_measure: String,
	pub min_value: f64,
	pub max_value: f64,
	pub status: String,
	pub src_type: String,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
pub struct StringValue {
	pub id: String,
	#[serde(rename = "type")]
	pub kind: String,
	#[serde(deserialize_with = "u8_as_bool")]
	pub recordable: bool,
	#[serde(deserialize_with = "u8_as_bool")]
	pub writeable: bool,
	pub value: String,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
//#[serde(untagged)]
pub enum BoilerIndicator {
	#[serde(rename = "CH")]
	CentralHeating,
	#[serde(rename = "HW")]
	HotWater,
	#[serde(rename = "No")]
	Off,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct UiUpdate {
	#[serde(rename = "CTD", deserialize_with = "uiupdate_date_parse")]
	pub current_date: ZonedDateTime,
	#[serde(rename = "CTR")]
	pub control: String,
	#[serde(rename = "UMD")]
	pub user_mode: String,
	#[serde(rename = "MMT", deserialize_with = "str_as_f64")]
	pub manual_set_point: f64,
	#[serde(rename = "CPM")]
	pub clock_program: String,
	#[serde(rename = "CSP", deserialize_with = "str_as_f64")]
	pub current_switch_point: f64,
	#[serde(rename = "TOR", deserialize_with = "str_as_bool")]
	pub temp_override_active: bool,
	#[serde(rename = "TOD", deserialize_with = "str_as_f64")]
	pub temp_override_duration: f64,
	#[serde(rename = "TOT", deserialize_with = "str_as_f64")]
	pub temp_override_set_point: f64,
	#[serde(rename = "TSP", deserialize_with = "str_as_f64")]
	pub temp_set_point: f64,
	#[serde(rename = "IHT", deserialize_with = "str_as_f64")]
	pub in_house_temp: f64,
	#[serde(rename = "IHS")]
	pub in_house_status: String,
	pub das: String,
	pub tas: String,
	#[serde(rename = "HMD", deserialize_with = "str_as_bool")]
	pub holiday_mode_active: bool,
	pub ars: String,
	#[serde(rename = "FPA", deserialize_with = "str_as_bool")]
	pub fireplace_active: bool,
	#[serde(rename = "ESI", deserialize_with = "str_as_bool")]
	pub powersave_active: bool,
	#[serde(rename = "BAI")]
	pub boiler_indicator: BoilerIndicator,
	#[serde(rename = "BLE", deserialize_with = "str_as_bool")]
	pub boiler_lock_active: bool,
	#[serde(rename = "BBE", deserialize_with = "str_as_bool")]
	pub boiler_block_active: bool,
	#[serde(rename = "BMR", deserialize_with = "str_as_bool")]
	pub boiler_maintenance_active: bool,
	pub pmr: String,
	pub rs: String,
	#[serde(rename = "DHW", deserialize_with = "str_as_bool")]
	pub hot_water_active: bool,
	#[serde(rename = "HED_EN", deserialize_with = "str_as_bool")]
	pub hed_enabled: bool,
	#[serde(rename = "HED_DEV", deserialize_with = "str_as_bool")]
	pub hed_device_at_home: bool,
	pub fah: String,
	pub dot: String,
	pub hed_db: String,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
pub struct UiStatus {
	pub id: String,
	#[serde(rename = "type")]
	pub kind: String,
	#[serde(deserialize_with = "u8_as_bool")]
	pub recordable: bool,
	#[serde(deserialize_with = "u8_as_bool")]
	pub writeable: bool,
	pub value: UiUpdate,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingRaw {
	#[serde(rename = "d", deserialize_with = "recording_date_parse")]
	pub date: Option<Date>,
	#[serde(rename = "hw")]
	pub hot_water: f64,
	#[serde(rename = "ch")]
	pub heating: f64,
	#[serde(rename = "T", deserialize_with = "recording_temp_convert")]
	pub average_outdoor_temp: f64,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
pub struct Recording {
	pub date: Date,
	pub hot_water: f64,
	pub heating: f64,
	pub average_outdoor_temp: f64,
}

impl Recording {
	pub fn from_raw(raw: RecordingRaw) -> Option<Recording> {
		raw.date.map(|date| Recording {
			date,
			hot_water: raw.hot_water,
			heating: raw.heating,
			average_outdoor_temp: raw.average_outdoor_temp,
		})
	}
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Deserialize)]
pub struct GasUsage {
	pub id: String,
	#[serde(rename = "type")]
	pub kind: String,
	#[serde(deserialize_with = "u8_as_bool")]
	pub recordable: bool,
	#[serde(deserialize_with = "u8_as_bool")]
	pub writeable: bool,
	pub value: Vec<RecordingRaw>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize)]
pub struct ValuePut {
	pub value: RawCommandArgument,
}

fn u8_as_bool<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
	u8::deserialize(d).map(|i| i != 0)
}

fn str_as_f64<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
	String::deserialize(d).map(|s| s.parse().expect("Cannot parse str as f64"))
}

fn str_as_bool<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
	String::deserialize(d).map(|s| s == "true")
}

fn recording_temp_convert<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
	i32::deserialize(d).map(|t| t as f64 / 10.)
}

fn uiupdate_date_parse<'de, D: serde::Deserializer<'de>>(d: D) -> Result<ZonedDateTime, D::Error> {
	let s = String::deserialize(d)?;
	let (datetime, _) = s
		.rsplit_once(' ')
		.filter(|(_, weekday)| weekday.len() == 2)
		.ok_or_else(|| D::Error::custom("expected a two-letter weekday after the thermostat timestamp"))?;
	let pieces = Pieces::parse(datetime).map_err(D::Error::custom)?;
	let time = pieces.time().ok_or_else(|| D::Error::custom("missing thermostat time"))?;
	let offset = pieces
		.to_numeric_offset()
		.ok_or_else(|| D::Error::custom("missing thermostat UTC offset"))?;
	pieces
		.date()
		.to_datetime(time)
		.to_zoned_datetime(TimeZone::fixed(offset))
		.map_err(D::Error::custom)
}

fn recording_date_parse<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Date>, D::Error> {
	let str_date = String::deserialize(d)?;
	if str_date == "255-256-65535" {
		// placeholder date when there is no entry for that day
		Ok(None)
	} else {
		Date::strptime("%d-%m-%Y", &str_date).map(Some).map_err(D::Error::custom)
	}
}

pub mod get {
	#![allow(non_upper_case_globals)]

	use super::*;

	pub const system_pressure: Command<FloatValue> = Command(
		RawCommand::Get(Cow::Borrowed("/system/appliance/systemPressure")),
		marker::PhantomData,
	);
	pub const display_code: Command<StringValue> = Command(
		RawCommand::Get(Cow::Borrowed("/system/appliance/displaycode")),
		marker::PhantomData,
	);
	pub const cause_code: Command<FloatValue> = Command(
		RawCommand::Get(Cow::Borrowed("/system/appliance/causecode")),
		marker::PhantomData,
	);
	pub const latitude: Command<StringValue> = Command(
		RawCommand::Get(Cow::Borrowed("/system/location/latitude")),
		marker::PhantomData,
	);
	pub const longitude: Command<StringValue> = Command(
		RawCommand::Get(Cow::Borrowed("/system/location/longitude")),
		marker::PhantomData,
	);
	pub const outdoor_temp: Command<OutdoorTempValue> = Command(
		RawCommand::Get(Cow::Borrowed("/system/sensors/temperatures/outdoor_t1")),
		marker::PhantomData,
	);
	pub const supply_temp: Command<FloatValue> = Command(
		RawCommand::Get(Cow::Borrowed("/heatingCircuits/hc1/actualSupplyTemperature")),
		marker::PhantomData,
	);
	pub const user_mode: Command<StringValue> = Command(
		RawCommand::Get(Cow::Borrowed("/heatingCircuits/hc1/usermode")),
		marker::PhantomData,
	);
	pub const status: Command<UiStatus> = Command(RawCommand::Get(Cow::Borrowed("/ecus/rrc/uiStatus")), marker::PhantomData);
	pub const gas_usage_entry_count: Command<FloatValue> = Command(
		RawCommand::Get(Cow::Borrowed("/ecus/rrc/recordings/gasusagePointer")),
		marker::PhantomData,
	);

	pub fn gas_usage_page(page_num: usize) -> Command<GasUsage> {
		assert!(page_num >= 1, "page_num starts with 1");
		Command(
			RawCommand::Get(Cow::from(format!("/ecus/rrc/recordings/gasusage?page={page_num}"))),
			marker::PhantomData,
		)
	}
}

pub mod put {
	#![allow(non_upper_case_globals)]

	use super::*;

	pub fn set_temp_room_manual(temp: f64) -> Command<()> {
		Command(
			RawCommand::Put(Cow::from("/heatingCircuits/hc1/temperatureRoomManual"), temp.into()),
			marker::PhantomData,
		)
	}

	pub fn set_manual_temp_override(temp: f64) -> Command<()> {
		Command(
			RawCommand::Put(Cow::from("/heatingCircuits/hc1/manualTempOverride/temperature"), temp.into()),
			marker::PhantomData,
		)
	}

	pub fn enable_manual_temp_override(enable: bool) -> Command<()> {
		Command(
			RawCommand::Put(
				Cow::from("/heatingCircuits/hc1/manualTempOverride/status"),
				if enable {
					"on"
				} else {
					"off"
				}
				.into(),
			),
			marker::PhantomData,
		)
	}
}

#[cfg(test)]
mod tests {
	use jiff::Timestamp;
	use jiff::civil::date;
	use jiff::tz::offset;

	use super::*;

	#[test]
	fn thermostat_timestamp_keeps_its_offset() {
		let mut deserializer = serde_json::Deserializer::from_str(r#""2024-02-29T20:49:54.123+01:00 Th""#);
		let datetime = uiupdate_date_parse(&mut deserializer).unwrap();
		assert_eq!(datetime.offset(), offset(1));
		assert_eq!(datetime.timestamp(), "2024-02-29T19:49:54.123Z".parse::<Timestamp>().unwrap());

		let mut deserializer = serde_json::Deserializer::from_str(r#""2024-02-29T19:49:54.123Z Th""#);
		let utc = uiupdate_date_parse(&mut deserializer).unwrap();
		assert_eq!(utc.offset(), offset(0));
		assert_eq!(utc.timestamp(), datetime.timestamp());
	}

	#[test]
	fn malformed_thermostat_timestamps_return_errors() {
		for input in [r#""""#, r#""éé""#, r#""2024-02-29T20:49:54 Th""#] {
			let mut deserializer = serde_json::Deserializer::from_str(input);
			assert!(uiupdate_date_parse(&mut deserializer).is_err(), "{input}");
		}
	}

	#[test]
	fn gas_recording_dates_are_parsed_and_placeholders_are_skipped() {
		let raw: RecordingRaw = serde_json::from_str(r#"{"d":"29-02-2024","hw":1.0,"ch":2.0,"T":35}"#).unwrap();
		assert_eq!(Recording::from_raw(raw).unwrap().date, date(2024, 2, 29));

		let raw: RecordingRaw = serde_json::from_str(r#"{"d":"255-256-65535","hw":0.0,"ch":0.0,"T":0}"#).unwrap();
		assert!(Recording::from_raw(raw).is_none());
	}
}
