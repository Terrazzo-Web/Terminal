use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Sub;
use std::ops::SubAssign;
use std::time::Duration;
use std::time::TryFromFloatSecsError;

use web_sys::js_sys::Date;

#[derive(Debug)]
pub struct DateTime(Date);

impl DateTime {
    pub fn now() -> Self {
        Self(Date::new_0())
    }

    pub fn from_utc(utc: Duration) -> Self {
        let t = Date::new_0();
        t.set_time(utc.as_secs_f64() * 1000.);
        Self(t)
    }

    pub fn utc(&self) -> Duration {
        Duration::from_secs_f64(self.epoch_millis() / 1000.)
    }

    pub fn to_start_of_day(&self) -> DateTime {
        let timestamp = self.clone();
        timestamp.0.set_hours(0);
        timestamp.0.set_minutes(0);
        timestamp.0.set_seconds(0);
        timestamp.0.set_milliseconds(0);
        return timestamp;
    }

    pub fn hour_minute(&self) -> String {
        format!("{:0>2}:{:0>2}", self.0.get_hours(), self.0.get_minutes())
    }

    pub fn day_month_year(&self) -> String {
        format!(
            "{:0>2}.{:0>2}.{}",
            self.0.get_date(),
            self.0.get_month() as u8,
            self.0.get_full_year()
        )
    }
}

impl DateTime {
    fn epoch_millis(&self) -> f64 {
        self.0.get_time()
    }
}

impl Clone for DateTime {
    fn clone(&self) -> Self {
        let copy = Date::new_0();
        copy.set_time(self.0.get_time());
        Self(copy)
    }
}

impl PartialEq for DateTime {
    fn eq(&self, other: &Self) -> bool {
        self.epoch_millis() == other.epoch_millis()
    }
}

impl Eq for DateTime {}

impl Sub for &DateTime {
    type Output = Result<Duration, TryFromFloatSecsError>;

    fn sub(self, rhs: Self) -> Self::Output {
        let millis = self.epoch_millis() - rhs.epoch_millis();
        debug_assert! { millis >= 0., "Negative duration between {self:?} and {rhs:?}" };
        Duration::try_from_secs_f64(millis / 1000.)
    }
}

impl Add<Duration> for &DateTime {
    type Output = DateTime;

    fn add(self, rhs: Duration) -> Self::Output {
        let mut result = self.clone();
        result += rhs;
        return result;
    }
}

impl Sub<Duration> for &DateTime {
    type Output = DateTime;

    fn sub(self, rhs: Duration) -> Self::Output {
        let mut result = self.clone();
        result -= rhs;
        return result;
    }
}

impl AddAssign<Duration> for DateTime {
    fn add_assign(&mut self, rhs: Duration) {
        self.0.set_time(self.0.get_time() + rhs.as_millis() as f64);
    }
}

impl SubAssign<Duration> for DateTime {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0.set_time(self.0.get_time() - rhs.as_millis() as f64);
    }
}

// wasm-pack test --firefox --headless --no-default-features --features client --lib
/*
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use wasm_bindgen_test::wasm_bindgen_test;
    use wasm_bindgen_test::wasm_bindgen_test_configure;

    use super::DateTime;

    wasm_bindgen_test_configure!(run_in_browser);
    #[wasm_bindgen_test]
    fn date() {
        let now = DateTime::from_utc(Duration::from_millis(1750440814997));
        assert_eq!(
            "20.05.2025 -- 19:33",
            format!("{} -- {}", now.day_month_year(), now.hour_minute())
        );
        let sod = now.to_start_of_day();
        assert_ne!(now, sod);
        assert_eq!(now, now);
        assert_eq!(
            "20.05.2025 -- 19:33",
            format!("{} -- {}", now.day_month_year(), now.hour_minute())
        );
        assert_eq!(
            "20.05.2025 -- 00:00",
            format!("{} -- {}", sod.day_month_year(), sod.hour_minute())
        );
    }
}
*/
