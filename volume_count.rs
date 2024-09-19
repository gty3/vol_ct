use databento::dbn::Mbp10Msg;
use serde_json::Value;
use std::collections::VecDeque;

// Struct to track trading actions and maintain ratios
pub struct ActionTracker {
    last_trade_mbp: Option<Mbp10Msg>, // Changed field name
    recent_bid_ratios: VecDeque<f64>,
    recent_ask_ratios: VecDeque<f64>,
    max_values: usize,
    recent_buy_sizes: VecDeque<u32>,
    recent_sell_sizes: VecDeque<u32>,
}

impl ActionTracker {
    pub fn new() -> Self {
        Self {
            last_trade_mbp: None,
            recent_bid_ratios: VecDeque::new(),
            recent_ask_ratios: VecDeque::new(),
            max_values: 200 // Moving average window size,
            recent_buy_sizes: VecDeque::new(),
            recent_sell_sizes: VecDeque::new(),
        }
    }

    pub fn process(&mut self, mbp: &Mbp10Msg) -> (Option<f64>, Option<f64>) {
        let mut bid_trade_ratio = None;
        let mut ask_trade_ratio = None;

        // Skip processing if last_trade_mbp exists and current mbp is a trade
        if self.last_trade_mbp.is_some() && mbp.action == 84 {
            // edge case where a trade is followed by another trade, will encorporate calculation later
            return (None, None);
        }

        if let Some(last_trade_mbp) = &self.last_trade_mbp {
            if mbp.action == 77 {
                return (None, None);
            }
            // Calculate bid_trade_ratio
            if last_trade_mbp.side == 65 {
                let bid_px_same = last_trade_mbp.levels[0].bid_px == mbp.levels[0].bid_px;
                let bid_sz_traded = if bid_px_same {
                    last_trade_mbp.levels[0]
                        .bid_sz
                        .saturating_sub(mbp.levels[0].bid_sz)
                } else {
                    last_trade_mbp.levels[0].bid_sz
                };
                let bid_ct_traded = if bid_px_same {
                    last_trade_mbp.levels[0]
                        .bid_ct
                        .saturating_sub(mbp.levels[0].bid_ct)
                } else {
                    last_trade_mbp.levels[0].bid_ct
                };
                if bid_ct_traded > 0 && bid_sz_traded > 0 {
                    bid_trade_ratio = Some((bid_sz_traded as f64 / bid_ct_traded as f64).max(0.01));
                }
            }

            // Calculate ask_trade_ratio
            if last_trade_mbp.side == 66 {
                let ask_px_same = last_trade_mbp.levels[0].ask_px == mbp.levels[0].ask_px;
                let ask_sz_traded = if ask_px_same {
                    last_trade_mbp.levels[0]
                        .ask_sz
                        .saturating_sub(mbp.levels[0].ask_sz)
                } else {
                    last_trade_mbp.levels[0].ask_sz
                };
                let ask_ct_traded = if ask_px_same {
                    last_trade_mbp.levels[0]
                        .ask_ct
                        .saturating_sub(mbp.levels[0].ask_ct)
                } else {
                    last_trade_mbp.levels[0].ask_ct
                };
                if ask_ct_traded > 0 && ask_sz_traded > 0 {
                    ask_trade_ratio = Some((ask_sz_traded as f64 / ask_ct_traded as f64).max(0.01));
                }
            }

            self.last_trade_mbp = None; // Clear after processing
        }

        // Update last_trade_mbp if current message is a trade (action 'T', which is 84)
        if mbp.action == 84 {
            self.last_trade_mbp = Some(mbp.clone());
            // Capture sizes for buy or sell trades
            match mbp.side {
                66 => {
                    // 'B' for Buy
                    println!("buy size: {}", mbp.size);
                    self.recent_buy_sizes.push_back(mbp.size);
                    if self.recent_buy_sizes.len() > self.max_values {
                        self.recent_buy_sizes.pop_front();
                    }
                }
                65 => {
                    // 'A' for Sell
                    self.recent_sell_sizes.push_back(mbp.size);
                    if self.recent_sell_sizes.len() > self.max_values {
                        self.recent_sell_sizes.pop_front();
                    }
                }
                _ => {}
            }
        }

        if let Some(ratio) = bid_trade_ratio {
            self.recent_bid_ratios.push_back(ratio);
            if self.recent_bid_ratios.len() > self.max_values {
                self.recent_bid_ratios.pop_front();
            }
        }

        if let Some(ratio) = ask_trade_ratio {
            self.recent_ask_ratios.push_back(ratio);
            if self.recent_ask_ratios.len() > self.max_values {
                self.recent_ask_ratios.pop_front();
            }
        }

        (bid_trade_ratio, ask_trade_ratio)
    }

    pub fn add_to_json(&self, mbp_json: &mut Value) {
        if !self.recent_bid_ratios.is_empty() {
            let avg_bid_ratio = self.recent_bid_ratios.iter().sum::<f64>() / self.recent_bid_ratios.len() as f64;
            mbp_json["bid_vol_ct"] = Value::Number(serde_json::Number::from_f64(avg_bid_ratio).unwrap());
        }

        if !self.recent_ask_ratios.is_empty() {
            let avg_ask_ratio =
                self.recent_ask_ratios.iter().sum::<f64>() / self.recent_ask_ratios.len() as f64;
            mbp_json["ask_vol_ct"] =
                Value::Number(serde_json::Number::from_f64(avg_ask_ratio).unwrap());
        }

        if !self.recent_buy_sizes.is_empty() {
            let avg_buy_size = self.recent_buy_sizes.iter().sum::<u32>() as f64
                / self.recent_buy_sizes.len() as f64;
            println!("Average Buy Size: {}", avg_buy_size);
            mbp_json["buy_vol_ct"] =
                Value::Number(serde_json::Number::from_f64(avg_buy_size).unwrap());
            println!("{:?}", Value::Number(serde_json::Number::from_f64(avg_buy_size).unwrap()));
        }

        if !self.recent_sell_sizes.is_empty() {
            let avg_sell_size = self.recent_sell_sizes.iter().sum::<u32>() as f64
                / self.recent_sell_sizes.len() as f64;
            mbp_json["sell_vol_ct"] =
                Value::Number(serde_json::Number::from_f64(avg_sell_size).unwrap());
        }
    }
}
