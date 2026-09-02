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
//! lookback, whether the close is above a reference moving average, and whether
//! the symbol stands on a point-and-figure buy signal.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use wickra_backtest_core::Candle;
use wickra_core::{
    BarBuilder, CrossSection as CoreCrossSection, Indicator, Member, PointAndFigureBars, Sma,
};

use crate::error::{Error, Result};

/// Default lookback for `new_high` / `new_low`: a year of weekly bars, the
/// convention the new-highs/new-lows family is quoted on.
const DEFAULT_PERIOD: usize = 52;

/// Default moving average for `above_ma`, the period the `% above MA` breadth
/// reading is conventionally quoted on.
const DEFAULT_MA_PERIOD: usize = 200;

/// Default point-and-figure box, as a fraction of the symbol's first close.
///
/// The builder takes an absolute box size, which cannot be one number across a
/// universe of mixed price levels: a box that resolves a 40-dollar name is noise
/// on a 40-thousand-dollar one. A fraction converts per symbol.
const DEFAULT_PNF_BOX: f64 = 0.01;

/// Default point-and-figure reversal, in boxes. Three is the classic setting the
/// Bullish Percent Index is quoted on.
const DEFAULT_PNF_REVERSAL: usize = 3;

/// How the cross-section the screener builds for itself is parameterised.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct BreadthSpec {
    /// Lookback for `new_high` / `new_low`, in bars. Defaults to 52.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<usize>,
    /// Period of the moving average `above_ma` compares the close against.
    /// Defaults to 200.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ma_period: Option<usize>,
    /// Point-and-figure box size for `on_buy_signal`, as a fraction of the
    /// symbol's first close. Defaults to 0.01, one percent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pnf_box: Option<f64>,
    /// Point-and-figure reversal in boxes for `on_buy_signal`. Defaults to 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pnf_reversal: Option<usize>,
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

    /// The point-and-figure box, as a fraction of the first close.
    fn pnf_box(&self) -> f64 {
        self.pnf_box.unwrap_or(DEFAULT_PNF_BOX)
    }

    /// The point-and-figure reversal, in boxes.
    fn pnf_reversal(&self) -> usize {
        self.pnf_reversal.unwrap_or(DEFAULT_PNF_REVERSAL)
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
        if self.pnf_box.is_some_and(|b| !b.is_finite() || b <= 0.0) {
            return Err(Error::BadSpec(
                "breadth pnf_box must be a positive fraction of price".into(),
            ));
        }
        if self.pnf_reversal == Some(0) {
            return Err(Error::BadSpec(
                "breadth pnf_reversal must be greater than 0".into(),
            ));
        }
        Ok(())
    }
}

/// Whether a symbol stands on a point-and-figure buy signal.
///
/// The classic reading: a buy signal starts when price breaks above the top of
/// the previous column of Xs (a double-top breakout) and ends when it breaks
/// below the bottom of the previous column of Os (a double-bottom breakdown).
///
/// `PointAndFigureBars` is what decides where a column ends, and it returns each
/// column as it completes; the breakout itself is then a comparison of the close
/// against the last completed column of that direction. No point-and-figure logic
/// is reimplemented here — the builder owns the reversal rule, and this owns only
/// the signal read off its output.
struct BuySignal {
    columns: PointAndFigureBars,
    last_x_high: Option<f64>,
    last_o_low: Option<f64>,
    on: bool,
}

impl BuySignal {
    /// Build the state, sizing the box off the symbol's first close.
    fn new(spec: &BreadthSpec, first_close: f64) -> Result<Self> {
        let box_size = first_close.abs() * spec.pnf_box();
        let columns = PointAndFigureBars::new(box_size, spec.pnf_reversal())
            .map_err(|e| Error::BadSpec(format!("breadth point-and-figure: {e}")))?;
        Ok(Self {
            columns,
            last_x_high: None,
            last_o_low: None,
            on: false,
        })
    }

    /// Fold one bar and return whether the symbol is on a buy signal after it.
    fn update(&mut self, candle: &Candle) -> bool {
        let Ok(core) = candle.to_core() else {
            // A bar the core rejects cannot move a column; the standing signal
            // is the honest answer for it.
            return self.on;
        };
        for column in self.columns.update(core) {
            if column.direction > 0 {
                self.last_x_high = Some(column.high);
            } else {
                self.last_o_low = Some(column.low);
            }
        }
        if self.last_x_high.is_some_and(|top| candle.close > top) {
            self.on = true;
        }
        // A breakdown is evaluated after a breakout so that, in the degenerate
        // case where a bar clears both references, the bearish reading wins.
        if self.last_o_low.is_some_and(|bottom| candle.close < bottom) {
            self.on = false;
        }
        self.on
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
    spec: BreadthSpec,
    // Built on the first bar: the box is a fraction of that bar's close, which
    // is not known until it arrives.
    buy_signal: Option<BuySignal>,
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
            spec: spec.clone(),
            buy_signal: None,
        })
    }

    /// Fold one bar and return the member this symbol contributes at it.
    ///
    /// The new-high test is against the window *before* this bar, so the first
    /// bar of a series is not trivially a new high, and `above_ma` stays false
    /// until the average is warm rather than defaulting to "above".
    pub(crate) fn update(&mut self, candle: &Candle) -> Result<Member> {
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

        if self.buy_signal.is_none() {
            self.buy_signal = Some(BuySignal::new(&self.spec, candle.close)?);
        }
        let on_buy_signal = self
            .buy_signal
            .as_mut()
            .expect("just built if absent")
            .update(candle);

        let change = self.prev_close.map_or(0.0, |prev| candle.close - prev);
        self.prev_close = Some(candle.close);

        Ok(Member::with_signals(
            change,
            candle.volume.max(0.0),
            new_high,
            new_low,
            self.above_ma,
            on_buy_signal,
        ))
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
        let first = state.update(&candle(0, 11.0, 9.0, 10.0)).unwrap();
        assert!(
            (first.change - 0.0).abs() < f64::EPSILON,
            "the first bar has no previous close to change against"
        );
        let second = state.update(&candle(1, 13.0, 11.0, 12.0)).unwrap();
        assert!((second.change - 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_first_bar_is_not_a_new_high_or_low() {
        let mut state = BreadthState::new(&BreadthSpec::default()).unwrap();
        let first = state.update(&candle(0, 11.0, 9.0, 10.0)).unwrap();
        assert!(!first.new_high);
        assert!(!first.new_low);
    }

    #[test]
    fn a_new_extreme_is_flagged_against_the_earlier_window() {
        let mut state = BreadthState::new(&BreadthSpec::default()).unwrap();
        state.update(&candle(0, 11.0, 9.0, 10.0)).unwrap();
        let up = state.update(&candle(1, 12.0, 10.0, 11.5)).unwrap();
        assert!(up.new_high);
        assert!(!up.new_low);

        let down = state.update(&candle(2, 11.0, 8.0, 9.0)).unwrap();
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
        state.update(&candle(0, 50.0, 40.0, 45.0)).unwrap();
        state.update(&candle(1, 12.0, 10.0, 11.0)).unwrap();
        state.update(&candle(2, 13.0, 11.0, 12.0)).unwrap();
        let after = state.update(&candle(3, 14.0, 12.0, 13.0)).unwrap();
        assert!(after.new_high, "the tall bar has left the window");
    }

    #[test]
    fn above_ma_stays_false_until_the_average_is_warm() {
        let spec = BreadthSpec {
            ma_period: Some(3),
            ..BreadthSpec::default()
        };
        let mut state = BreadthState::new(&spec).unwrap();
        assert!(!state.update(&candle(0, 11.0, 9.0, 10.0)).unwrap().above_ma);
        assert!(!state.update(&candle(1, 12.0, 10.0, 11.0)).unwrap().above_ma);
        // Third bar: the 3-bar average is 11, and the close is 12.
        assert!(state.update(&candle(2, 13.0, 11.0, 12.0)).unwrap().above_ma);
        // A close back under the average clears it again.
        assert!(!state.update(&candle(3, 10.0, 8.0, 9.0)).unwrap().above_ma);
    }

    #[test]
    fn a_zero_period_is_refused() {
        let spec = BreadthSpec {
            period: Some(0),
            ..BreadthSpec::default()
        };
        assert!(spec.validate().is_err());
        let spec = BreadthSpec {
            ma_period: Some(0),
            ..BreadthSpec::default()
        };
        assert!(spec.validate().is_err());
        assert!(BreadthSpec::default().validate().is_ok());
    }

    /// Walk a price path one close at a time, returning the buy-signal flag
    /// after each bar.
    fn buy_signals(path: &[f64], spec: &BreadthSpec) -> Vec<bool> {
        let mut state = BreadthState::new(spec).unwrap();
        path.iter()
            .enumerate()
            .map(|(i, close)| {
                let time = i64::try_from(i).unwrap();
                state
                    .update(&candle(time, close + 1.0, close - 1.0, *close))
                    .unwrap()
                    .on_buy_signal
            })
            .collect()
    }

    /// A leg up, a reversal down, then a break above the first leg's top is the
    /// classic double-top breakout; a later break below the intervening column's
    /// bottom takes the signal away again.
    #[test]
    fn the_buy_signal_follows_the_point_and_figure_columns() {
        let spec = BreadthSpec {
            // A one-point box on a hundred-point price, three boxes to reverse.
            pnf_box: Some(0.01),
            pnf_reversal: Some(3),
            ..BreadthSpec::default()
        };

        let mut path: Vec<f64> = Vec::new();
        let mut push = |from: i32, to: i32| {
            let step = if to >= from { 1 } else { -1 };
            let mut price = from;
            while price != to {
                path.push(f64::from(price));
                price += step;
            }
            path.push(f64::from(to));
        };
        push(100, 120); // first leg up
        push(120, 104); // reversal down: completes the X column at 120
        push(104, 126); // break above 120: the buy signal starts here
        push(126, 95); // completes an O column and breaks below 104

        let signals = buy_signals(&path, &spec);

        // Nothing can be on before a column has completed and been exceeded.
        assert!(!signals[0]);
        assert!(
            signals.iter().any(|on| *on),
            "a double-top breakout must turn the signal on"
        );
        assert!(
            !signals[signals.len() - 1],
            "a break below the previous column of Os must take it away"
        );

        // The turn-on is on the way up past 120, not before.
        let first_on = signals.iter().position(|on| *on).unwrap();
        assert!(
            path[first_on] > 120.0,
            "turned on at {} before clearing the previous column top",
            path[first_on]
        );
    }

    /// A price path that never reverses completes no column, so no double top
    /// has been made and no signal has been given. Reporting one would be an
    /// invention, not a reading.
    #[test]
    fn a_path_that_never_reverses_gives_no_signal() {
        let path: Vec<f64> = (0..60).map(|i| 100.0 + f64::from(i)).collect();
        assert!(buy_signals(&path, &BreadthSpec::default())
            .iter()
            .all(|on| !*on));
    }

    #[test]
    fn a_nonsensical_point_and_figure_configuration_is_refused() {
        let spec = BreadthSpec {
            pnf_box: Some(0.0),
            ..BreadthSpec::default()
        };
        assert!(spec.validate().is_err());
        let spec = BreadthSpec {
            pnf_box: Some(f64::NAN),
            ..BreadthSpec::default()
        };
        assert!(spec.validate().is_err());
        let spec = BreadthSpec {
            pnf_reversal: Some(0),
            ..BreadthSpec::default()
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn an_empty_panel_is_not_a_cross_section() {
        assert!(assemble(Vec::new(), 0).is_none());
        assert!(assemble(vec![Member::new(1.0, 10.0, false, false)], 0).is_some());
    }
}
