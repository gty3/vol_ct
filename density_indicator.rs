use databento::dbn::Mbp10Msg;
use std::collections::VecDeque;
use crate::ExtendedMbp10Msg;

// Struct to track trading actions and maintain ratios
pub struct ActionTracker {
    last_trade_mbp: Option<Mbp10Msg>,
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
            max_values: 200,
            recent_buy_sizes: VecDeque::new(),
            recent_sell_sizes: VecDeque::new(),
        }
    }

    pub fn process(&mut self, mbp: &Mbp10Msg) -> ExtendedMbp10Msg {
        let mut bid_trade_ratio = None;
        let mut ask_trade_ratio = None;

        // Skip processing if last_trade_mbp exists and current mbp is a trade
        if self.last_trade_mbp.is_some() && mbp.action == 84 {
            // Edge case where a trade is followed by another trade
            return ExtendedMbp10Msg {
                mbp10: mbp.clone(),
                initial: false,
                bid_density: None,
                ask_density: None,
                buy_density: None,
                sell_density: None,
            };
        }

        if let Some(last_trade_mbp) = &self.last_trade_mbp {
            // Ignore synthetic trades
            if mbp.action == 77 {
                return ExtendedMbp10Msg {
                    mbp10: mbp.clone(),
                    initial: false,
                    bid_density: None,
                    ask_density: None,
                    buy_density: None,
                    sell_density: None,
                };
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

        // Calculate average densities
        let avg_bid_density = if !self.recent_bid_ratios.is_empty() {
            Some(
                self.recent_bid_ratios.iter().sum::<f64>() / self.recent_bid_ratios.len() as f64,
            )
        } else {
            None
        };

        let avg_ask_density = if !self.recent_ask_ratios.is_empty() {
            Some(
                self.recent_ask_ratios.iter().sum::<f64>() / self.recent_ask_ratios.len() as f64,
            )
        } else {
            None
        };

        let avg_buy_density = if !self.recent_buy_sizes.is_empty() {
            Some(
                self.recent_buy_sizes.iter().sum::<u32>() as f64 / self.recent_buy_sizes.len() as f64,
            )
        } else {
            None
        };  

        let avg_sell_density = if !self.recent_sell_sizes.is_empty() {
            Some(
                self.recent_sell_sizes.iter().sum::<u32>() as f64 / self.recent_sell_sizes.len() as f64,
            )
        } else {
            None
        };
        

        // Construct mbp_with_ratios
        ExtendedMbp10Msg {
            mbp10: mbp.clone(),
            initial: false, // Set appropriately based on your logic
            bid_density: avg_bid_density,
            ask_density: avg_ask_density,
            buy_density: avg_buy_density,
            sell_density: avg_sell_density,
        }
    }
}

// Add this implementation
impl Default for ActionTracker {
    fn default() -> Self {
        Self::new()
    }
}
