use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
pub struct LocalTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub subsec: f32,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub weekday: usize,
}

impl LocalTime {
    pub fn now() -> Self {
        let time = unsafe { libc::time(std::ptr::null_mut()) };
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let mut result = unsafe { libc::localtime_r(&time, &mut tm) };
        if result.is_null() {
            result = unsafe { libc::gmtime_r(&time, &mut tm) };
        }

        let subsec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos() as f32 / 1_000_000_000.0)
            .unwrap_or(0.0);

        if result.is_null() {
            return Self {
                hour: 0,
                minute: 0,
                second: 0,
                subsec,
                year: 1970,
                month: 1,
                day: 1,
                weekday: 4,
            };
        }

        Self {
            hour: tm.tm_hour as u32,
            minute: tm.tm_min as u32,
            second: tm.tm_sec as u32,
            subsec,
            year: tm.tm_year + 1900,
            month: (tm.tm_mon + 1) as u32,
            day: tm.tm_mday as u32,
            weekday: tm.tm_wday.clamp(0, 6) as usize,
        }
    }

    pub fn clock_text(
        &self,
        twelve_hour: bool,
        show_seconds: bool,
    ) -> (String, Option<&'static str>) {
        let (hour, meridiem) = if twelve_hour {
            let hour = self.hour % 12;
            let hour = if hour == 0 { 12 } else { hour };
            (hour, Some(if self.hour < 12 { "AM" } else { "PM" }))
        } else {
            (self.hour, None)
        };

        let text = if show_seconds {
            format!("{hour:02}:{:02}:{:02}", self.minute, self.second)
        } else {
            format!("{hour:02}:{:02}", self.minute)
        };

        (text, meridiem)
    }

    pub fn date_text(&self) -> String {
        const WEEKDAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
        const MONTHS: [&str; 12] = [
            "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
        ];
        let weekday = WEEKDAYS.get(self.weekday).copied().unwrap_or("---");
        let month = MONTHS
            .get(self.month.saturating_sub(1) as usize)
            .copied()
            .unwrap_or("---");
        format!("{weekday} · {:02} {month} {}", self.day, self.year)
    }

    pub fn minute_progress(&self) -> f32 {
        (self.second as f32 + self.subsec) / 60.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LocalTime {
        LocalTime {
            hour: 13,
            minute: 5,
            second: 9,
            subsec: 0.5,
            year: 2026,
            month: 11,
            day: 11,
            weekday: 3,
        }
    }

    #[test]
    fn formats_24_hour_time() {
        assert_eq!(
            sample().clock_text(false, true),
            ("13:05:09".to_string(), None)
        );
        assert_eq!(
            sample().clock_text(false, false),
            ("13:05".to_string(), None)
        );
    }

    #[test]
    fn formats_12_hour_time() {
        assert_eq!(
            sample().clock_text(true, true),
            ("01:05:09".to_string(), Some("PM"))
        );
        let mut midnight = sample();
        midnight.hour = 0;
        assert_eq!(
            midnight.clock_text(true, true),
            ("12:05:09".to_string(), Some("AM"))
        );
    }

    #[test]
    fn formats_date_and_progress() {
        assert_eq!(sample().date_text(), "WED · 11 NOV 2026");
        assert!((sample().minute_progress() - 9.5 / 60.0).abs() < 0.0001);
    }
}
