//! Building the market cross-section out of the universe itself.
//!
//! The breadth family reads a panel of the whole market at one bar — how many
//! symbols advanced, how many printed a new high, how much volume went each way.
//! A backtester has to be handed that panel, because it sees one instrument. A
//! screener already holds the whole universe, so it can assemble the panel from
//! what it is scanning, and a breadth screen needs no second data source.
//!
//! What a member carries comes from each symbol's own bar: the change against its
//! previous close, the bar volume, whether the bar set a new high or low over a
//! lookback, and whether the close is above a reference moving average. The one
//! signal that cannot be read off a candle is `on_buy_signal`, which needs a
//! point-and-figure column history; until that exists, a spec that depends on it
//! is refused rather than answered with a panel where the flag is false for
//! everyone.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use wickra_backtest_core::Candle;
use wickra_core::{CrossSection as CoreCrossSection, Indicator, Member, Sma};

use crate::error::{Error, Result};

/// Default lookback for `new_high` / `new_low`: a year of weekly bars, the
/// convention the new-highs/new-lows family is quoted on.
const DEFAULT_PERIOD: usize = 52;

/// Default moving average for `above_ma`, the period the `% above MA` breadth
/// reading is conventionally quoted on.
const DEFAULT_MA_PERIOD: usize = 200;

/// The indicator whose member signal the screener cannot yet derive.
pub(crate) const NEEDS_BUY_SIGNAL: &str = "BullishPercentIndex";

/// How the cross-section the screener builds for itself is parameterised.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct BreadthSpec {
    /// Lookback for `new_high` / `new_low`, in bars. Defaults to 52.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<usize>,
    /// Period of the moving average `above_ma` compares the close against.
    /// Defaults to 200.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ma_period: Option<usize>,
}

impl BreadthSpec {
    /// The lookback for new highs and lows.
    fn period(&self) -> usize {
        self.period.unwrap_or(DEFAULT_PERIOD)
    }

    /// The moving-average period for `above_ma`.
    fn ma_period(&self) -> usize {
        self.ma_period.unwrap_or(DEFAULT_MA_PERIOD)
    }

    /// Reject a nonsensical configuration up front rather than at the first bar.
    pub(crate) fn validate(&self) -> Result<()> {
        if self.period == Some(0) {
            return Err(Error::BadSpec(
                "breadth period must be greater than 0".into(),
            ));
        }
        if self.ma_period == Some(0) {
            return Err(Error::BadSpec(
                "breadth ma_period must be greater than 0".into(),
            ));
        }
        Ok(())
    }
}

/// One symbol's rolling state for the cross-section member it contributes.
pub(crate) struct BreadthState {
    prev_close: Option<f64>,
    highs: VecDeque<f64>,
    lows: VecDeque<f64>,
    period: usize,
    ma: Sma,
    above_ma: bool,
}

impl BreadthState {
    /// Build the state for a breadth configuration.
    pub(crate) fn new(spec: &BreadthSpec) -> Result<Self> {
        let ma = Sma::new(spec.ma_period())
            .map_err(|e| Error::BadSpec(format!("breadth ma_period: {e}")))?;
        Ok(Self {
            prev_close: None,
            highs: VecDeque::new(),
            lows: VecDeque::new(),
            period: spec.period(),
            ma,
            above_ma: false,
        })
    }

    /// Fold one bar and return the member this symbol contributes at it.
    ///
    /// The new-high test is against the window *before* this bar, so the first
    /// bar of a series is not trivially a new high, and `above_ma` stays false
    /// until the average is warm rather than defaulting to "above".
    pub(crate) fn update(&mut self, candle: &Candle) -> Member {
        let new_high = self.highs.iter().copied().fold(f64::NEG_INFINITY, f64::max) < candle.high
            && !self.highs.is_empty();
        let new_low = self.lows.iter().copied().fold(f64::INFINITY, f64::min) > candle.low
            && !self.lows.is_empty();

        self.highs.push_back(candle.high);
        self.lows.push_back(candle.low);
        if self.highs.len() > self.period {
            self.highs.pop_front();
            self.lows.pop_front();
        }

        if let Some(avg) = self.ma.update(candle.close) {
            self.above_ma = candle.close > avg;
        }

        let change = self.prev_close.map_or(0.0, |prev| candle.close - prev);
        self.prev_close = Some(candle.close);

        Member::with_signals(
            change,
            candle.volume.max(0.0),
            new_high,
            new_low,
            self.above_ma,
            // Derived from a point-and-figure column history, which this state
            // does not keep. A spec that reads it is refused, so no reader ever
            // sees this placeholder as if it were an answer.
            false,
        )
    }
}

/// Assemble the cross-section for one bar from the members that printed at it.
///
/// Returns `None` when nothing printed: a bar with no members is not a market
/// with zero breadth, it is a bar the universe has no reading for, and
/// `CrossSection::new` rejects an empty panel for the same reason.
pub(crate) fn assemble(members: Vec<Member>, timestamp: i64) -> Option<CoreCrossSection> {
    if members.is_empty() {
        return None;
    }
    CoreCrossSection::new(members, timestamp).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(time: i64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            time,
            open: close,
            high,
            low,
            close,
            volume: 100.0,
        }
    }

    #[test]
    fn change_is_against_the_previous_close() {
        let mut state = BreadthState::new(&BreadthSpec::default()).unwrap();
        let first = state.update(&candle(0, 11.0, 9.0, 10.0));
        assert!(
            (first.change - 0.0).abs() < f64::EPSILON,
            "the first bar has no previous close to change against"
        );
        let second = state.update(&candle(1, 13.0, 11.0, 12.0));
        assert!((second.change - 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_first_bar_is_not_a_new_high_or_low() {
        let mut state = BreadthState::new(&BreadthSpec::default()).unwrap();
        let first = state.update(&candle(0, 11.0, 9.0, 10.0));
        assert!(!first.new_high);
        assert!(!first.new_low);
    }

    #[test]
    fn a_new_extreme_is_flagged_against_the_earlier_window() {
        let mut state = BreadthState::new(&BreadthSpec::default()).unwrap();
        state.update(&candle(0, 11.0, 9.0, 10.0));
        let up = state.update(&candle(1, 12.0, 10.0, 11.5));
        assert!(up.new_high);
        assert!(!up.new_low);

        let down = state.update(&candle(2, 11.0, 8.0, 9.0));
        assert!(!down.new_high);
        assert!(down.new_low);
    }

    #[test]
    fn the_window_forgets_beyond_the_period() {
        let spec = BreadthSpec {
            period: Some(2),
            ..BreadthSpec::default()
        };
        let mut state = BreadthState::new(&spec).unwrap();
        // A tall bar, then two ordinary ones: the tall bar drops out of a
        // two-bar window, so an ordinary high becomes a new high again.
        state.update(&candle(0, 50.0, 40.0, 45.0));
        state.update(&candle(1, 12.0, 10.0, 11.0));
        state.update(&candle(2, 13.0, 11.0, 12.0));
        let after = state.update(&candle(3, 14.0, 12.0, 13.0));
        assert!(after.new_high, "the tall bar has left the window");
    }

    #[test]
    fn above_ma_stays_false_until_the_average_is_warm() {
        let spec = BreadthSpec {
            ma_period: Some(3),
            ..BreadthSpec::default()
        };
        let mut state = BreadthState::new(&spec).unwrap();
        assert!(!state.update(&candle(0, 11.0, 9.0, 10.0)).above_ma);
        assert!(!state.update(&candle(1, 12.0, 10.0, 11.0)).above_ma);
        // Third bar: the 3-bar average is 11, and the close is 12.
        assert!(state.update(&candle(2, 13.0, 11.0, 12.0)).above_ma);
        // A close back under the average clears it again.
        assert!(!state.update(&candle(3, 10.0, 8.0, 9.0)).above_ma);
    }

    #[test]
    fn a_zero_period_is_refused() {
        let spec = BreadthSpec {
            period: Some(0),
            ma_period: None,
        };
        assert!(spec.validate().is_err());
        let spec = BreadthSpec {
            period: None,
            ma_period: Some(0),
        };
        assert!(spec.validate().is_err());
        assert!(BreadthSpec::default().validate().is_ok());
    }

    #[test]
    fn an_empty_panel_is_not_a_cross_section() {
        assert!(assemble(Vec::new(), 0).is_none());
        assert!(assemble(vec![Member::new(1.0, 10.0, false, false)], 0).is_some());
    }
}
