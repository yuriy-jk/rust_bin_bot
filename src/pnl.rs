use ta_lib_wrapper::TA_Real;

pub fn count_pnl_wma(
    open: &Vec<f64>,
    high: &Vec<f64>,
    low: &Vec<f64>,
    sar_values: &Vec<TA_Real>,
    wma_values: &Vec<TA_Real>,
) -> (i32, i32, f64) {
    // INITIALIZE SATRT POSITION

    let mut pos: &str = if open[0] > sar_values[0] {
        "buy"
    } else {
        "sell"
    };

    // TRADE CONST
    let summ: f64 = 100.0;
    let mut size: f64 = 0.0;
    let mut position_profit: f64 = 0.0;
    let mut profit: f64 = 0.0;
    let mut profit_trade: i32 = 0;
    let mut loss_trade: i32 = 0;
    let mut offset: i32 = 0;

    // START PNL_COUNT ITERATION
    let count: usize = sar_values.len();
    for i in 1..count - 1 {
        let profit_line = wma_values[i] + (wma_values[i] - sar_values[i]);
        if pos == "sell" {
            if size > 0.0 {
                if open[i] > sar_values[i] {
                    // global change position signal with position size
                    position_profit = summ - (open[i + 1] * size); // count short profit
                    if position_profit > 0.0 {
                        profit_trade += 1
                    } else {
                        loss_trade += 1
                    };
                    profit += position_profit; // add profit
                    size = summ / open[i + 1]; //create buy size
                    pos = "buy"; // global make position

                    offset = 1;
                } else if low[i] < profit_line && offset == 0 {
                    //close local position "sell" and resume iteration without position
                    position_profit = summ - (open[i + 1] * size);
                    if position_profit > 0.0 {
                        profit_trade += 1
                    } else {
                        loss_trade += 1
                    };
                    profit += position_profit;
                    size = 0.0;
                } else {
                    offset = 0;
                }
            } else if size == 0.0 && open[i] > sar_values[i] {
                // global change position signal without position size
                size = summ / open[i + 1]; //create buy size
                pos = "buy"; // global make position
                offset = 1;
            }
        } else if pos == "buy" {
            if size > 0.0 {
                if open[i] < sar_values[i] {
                    position_profit = (open[i + 1] * size) - 100.0;
                    if position_profit > 0.0 {
                        profit_trade += 1
                    } else {
                        loss_trade += 1
                    };
                    profit += position_profit;
                    size = summ / open[i + 1];
                    pos = "sell";
                    offset = 1;
                } else if high[i] > profit_line && offset == 0 {
                    position_profit = (open[i + 1] * size) - 100.0;
                    if position_profit > 0.0 {
                        profit_trade += 1
                    } else {
                        loss_trade += 1
                    };
                    profit += position_profit;
                    size = 0.0;
                } else {
                    offset = 0;
                }
            } else if size == 0.0 && open[i] < sar_values[i] {
                size = summ / open[i + 1];
                pos = "sell";
                offset = 1;
            }
        }
    }
    (profit_trade, loss_trade, profit)
}
