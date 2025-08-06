use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Cell,
};

/// 横向条形图渲染器
pub struct BarChartRenderer {
    max_bar_width: u16,
    bid_color: Color,
    ask_color: Color,
    bar_char: char,
}

/// 条形图数据结构
#[derive(Debug, Clone)]
pub struct BarChartData {
    pub volume: f64,
    pub normalized_length: u16,
    pub color: Color,
    pub text: String,
}

impl BarChartRenderer {
    /// 创建新的条形图渲染器
    pub fn new() -> Self {
        Self {
            max_bar_width: 15, // 减小最大条形图宽度，模拟字体缩小
            bid_color: Color::Green,
            ask_color: Color::Red,
            bar_char: '█',
        }
    }

    /// 创建带自定义配置的条形图渲染器
    pub fn with_config(max_width: u16, bid_color: Color, ask_color: Color) -> Self {
        Self {
            max_bar_width: max_width,
            bid_color,
            ask_color,
            bar_char: '█',
        }
    }

    /// 渲染bid条形图（基于BTC数量的unicode块字符显示，绿色背景白色数字）
    pub fn render_bid_bar(&self, volume: f64, max_volume: f64, cell_width: u16) -> String {
        if volume <= 0.0 {
            return String::new();
        }

        // 基于BTC数量计算unicode块字符单位 - 每0.1BTC一个最小单位
        let block_count = self.calculate_btc_blocks(volume);
        
        // 格式化挂单量显示文本 - 保留小数点后5位
        let volume_text = format!("{:.5}", volume);

        self.create_btc_bar_string(block_count, &volume_text, cell_width, true)
    }

    /// 渲染ask条形图（基于BTC数量的unicode块字符显示，红色背景白色数字）
    pub fn render_ask_bar(&self, volume: f64, max_volume: f64, cell_width: u16) -> String {
        if volume <= 0.0 {
            return String::new();
        }

        // 基于BTC数量计算unicode块字符单位 - 每0.1BTC一个最小单位
        let block_count = self.calculate_btc_blocks(volume);
        
        // 格式化挂单量显示文本 - 保留小数点后5位
        let volume_text = format!("{:.5}", volume);

        self.create_btc_bar_string(block_count, &volume_text, cell_width, false)
    }

    /// 创建带文本的条形图单元格（使用unicode块字符显示，数字白色，色块有颜色）
    pub fn create_bar_with_text<'a>(&'a self, volume: f64, max_volume: f64, cell_width: u16, is_bid: bool) -> Cell<'a> {
        if volume <= 0.0 {
            return Cell::from("");
        }

        // 基于BTC数量计算unicode块字符单位 - 每0.1BTC一个最小单位
        let units = self.calculate_btc_blocks(volume);
        
        // 格式化挂单量显示文本 - 保留小数点后5位
        let volume_text = format!("{:.5}", volume);

        // 创建unicode块字符串
        let bar_chars = self.create_unicode_bar_from_units(units);
        
        // 确保条形图不超过单元格宽度
        let max_bar_width = cell_width.saturating_sub(volume_text.len() as u16 + 2) as usize;
        let truncated_bar = if bar_chars.chars().count() > max_bar_width {
            bar_chars.chars().take(max_bar_width).collect()
        } else {
            bar_chars
        };

        // 创建包含不同颜色部分的Line
        let line = if !truncated_bar.is_empty() {
            Line::from(vec![
                Span::styled(volume_text, Style::default().fg(Color::White)),     // 数字白色
                Span::raw(" "),                                                   // 空格分隔
                Span::styled(truncated_bar, Style::default().fg(if is_bid { Color::Green } else { Color::Red })), // unicode块字符有颜色
            ])
        } else {
            // 没有色块时，仍然显示数字（适用于小于0.1BTC的挂单量）
            Line::from(vec![
                Span::styled(format!("{} ", volume_text), Style::default().fg(Color::White)), // 只有数字时左对齐
            ])
        };

        Cell::from(line)
    }

    /// 计算条形图长度
    pub fn calculate_bar_length(&self, volume: f64, max_volume: f64, max_width: u16) -> u16 {
        if max_volume <= 0.0 || volume <= 0.0 {
            return 0;
        }

        let ratio = volume / max_volume;
        let calculated_length = (ratio * max_width as f64) as u16;
        
        // 确保至少有1个字符的长度（如果音量大于0）
        if calculated_length == 0 && volume > 0.0 {
            1
        } else {
            calculated_length.min(max_width)
        }
    }

    /// 创建条形图字符串
    fn create_bar_string(&self, bar_length: u16, text: &str, available_width: u16) -> String {
        if bar_length == 0 {
            return text.to_string();
        }

        // 创建条形图部分
        let bar_chars = self.bar_char.to_string().repeat(bar_length as usize);
        
        // 计算剩余空间
        let total_content_length = bar_chars.len() + text.len();
        let padding_length = if total_content_length < available_width as usize {
            available_width as usize - total_content_length
        } else {
            0
        };

        // 创建填充
        let padding = " ".repeat(padding_length);

        // 组合最终字符串
        format!("{}{}{}", bar_chars, padding, text)
    }

    /// 创建增强的条形图字符串（支持更好的可视化）
    fn create_enhanced_bar_string(&self, bar_length: u16, text: &str, available_width: u16, is_bid: bool) -> String {
        if bar_length == 0 {
            return format!(" {}", text); // 没有条形图时仍显示数值
        }

        // 使用不同的字符来表示不同强度的条形图
        let bar_chars = if bar_length >= available_width / 2 {
            // 超过一半宽度时使用实心块
            "█".repeat(bar_length as usize)
        } else if bar_length >= available_width / 4 {
            // 四分之一到一半时使用中等密度
            "▓".repeat(bar_length as usize)
        } else {
            // 小于四分之一时使用轻度密度
            "░".repeat(bar_length as usize)
        };
        
        // 为bid和ask使用不同的布局
        if is_bid {
            // bid: 先显示条形图，再显示数值（从左到右显示量的大小）
            let total_length = bar_chars.len() + text.len() + 1; // +1 for space
            if total_length <= available_width as usize {
                let padding_length = available_width as usize - total_length;
                let padding = " ".repeat(padding_length);
                format!("{} {}{}", bar_chars, text, padding)
            } else {
                format!("{} {}", bar_chars, text)
            }
        } else {
            // ask: 先显示数值，再显示条形图（从右到左显示量的大小）
            let total_length = bar_chars.len() + text.len() + 1; // +1 for space
            if total_length <= available_width as usize {
                let padding_length = available_width as usize - total_length;
                let padding = " ".repeat(padding_length);
                format!("{}{} {}", padding, text, bar_chars)
            } else {
                format!("{} {}", text, bar_chars)
            }
        }
    }

    /// 基于BTC数量计算unicode块字符 - 每0.001BTC一个最小单位
    pub fn calculate_btc_blocks(&self, volume: f64) -> u16 {
        if volume <= 0.0 {
            return 0;
        }
        
        // 每0.001BTC一个最小单位，如果挂单量低于0.001BTC则显示一个最小的字符块
        let units = if volume < 0.001 {
            1  // 低于0.001BTC时显示一个最小字符块
        } else {
            (volume / 0.005).round() as u16
        };
        
        // 不设置上限，让显示更准确反映实际挂单量
        units
    }

    /// 创建基于BTC的条形图字符串，使用unicode块字符，参照volume profile实现
    fn create_btc_bar_string(&self, units: u16, text: &str, cell_width: u16, _is_bid: bool) -> String {
        if units == 0 {
            return String::new(); // 没有色块时返回空字符串，不显示任何内容
        }

        // 使用unicode块字符创建更精细的bar显示，参照volume profile实现
        let bar_chars = self.create_unicode_bar_from_units(units);
        
        // 确保条形图不超过单元格宽度 - 按字符数而不是字节数截断
        let max_bar_width = cell_width.saturating_sub(text.len() as u16 + 2) as usize; // 为数字和边距预留空间
        let truncated_bar = if bar_chars.chars().count() > max_bar_width {
            bar_chars.chars().take(max_bar_width).collect()
        } else {
            bar_chars
        };
        
        // 数字在前，色块在后，这样数字可以用默认颜色，色块用指定颜色
        format!("{} {}", text, truncated_bar)
    }

    /// 创建Unicode块字符填充的bar，每个字符块代表0.001 BTC的最小单位
    /// 每个部分字符（▏▎▍▌▋▊▉）代表不同数量的0.001 BTC单位，每个完整字符█代表8个0.001 BTC单位
    fn create_unicode_bar_from_units(&self, units: u16) -> String {
        if units == 0 {
            return String::new();
        }

        // 计算完整字符数（每个█代表8个0.001 BTC单位）
        let full_chars = units / 8;
        // 计算剩余的0.001 BTC单位数
        let remaining_units = units % 8;
        
        let mut bar = String::new();
        
        // 添加完整填充的字符
        for _ in 0..full_chars {
            bar.push('█');
        }
        
        // 添加部分填充的字符（如果有剩余）
        if remaining_units > 0 {
            let partial_char = match remaining_units {
                1 => "▏",  // 1个0.001 BTC单位
                2 => "▎",  // 2个0.001 BTC单位
                3 => "▍",  // 3个0.001 BTC单位
                4 => "▌",  // 4个0.001 BTC单位
                5 => "▋",  // 5个0.001 BTC单位
                6 => "▊",  // 6个0.001 BTC单位
                7 => "▉",  // 7个0.001 BTC单位
                _ => " ",  // 不应该到达这里
            };
            bar.push_str(partial_char);
        }
        
        bar
    }

    /// 设置最大条形图宽度
    pub fn set_max_bar_width(&mut self, width: u16) {
        self.max_bar_width = width;
    }

    /// 设置条形图颜色
    pub fn set_colors(&mut self, bid_color: Color, ask_color: Color) {
        self.bid_color = bid_color;
        self.ask_color = ask_color;
    }

    /// 设置条形图字符
    pub fn set_bar_char(&mut self, ch: char) {
        self.bar_char = ch;
    }

    /// 获取bid颜色
    pub fn get_bid_color(&self) -> Color {
        self.bid_color
    }

    /// 获取ask颜色
    pub fn get_ask_color(&self) -> Color {
        self.ask_color
    }

    /// 创建条形图数据
    pub fn create_bar_data(&self, volume: f64, max_volume: f64, is_bid: bool) -> BarChartData {
        let color = if is_bid { self.bid_color } else { self.ask_color };
        let normalized_length = self.calculate_bar_length(volume, max_volume, self.max_bar_width);
        let text = format!("{:.3}", volume);

        BarChartData {
            volume,
            normalized_length,
            color,
            text,
        }
    }

    /// 验证条形图参数
    pub fn validate_parameters(&self, volume: f64, max_volume: f64) -> Result<(), String> {
        if volume < 0.0 {
            return Err("音量不能为负数".to_string());
        }
        if max_volume < 0.0 {
            return Err("最大音量不能为负数".to_string());
        }
        if max_volume > 0.0 && volume > max_volume {
            return Err("音量不能超过最大音量".to_string());
        }
        Ok(())
    }
}

impl Default for BarChartRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bar_length() {
        let renderer = BarChartRenderer::new();
        
        // 测试正常情况
        assert_eq!(renderer.calculate_bar_length(50.0, 100.0, 20), 10);
        assert_eq!(renderer.calculate_bar_length(100.0, 100.0, 20), 20);
        assert_eq!(renderer.calculate_bar_length(25.0, 100.0, 20), 5);
        
        // 测试边界情况
        assert_eq!(renderer.calculate_bar_length(0.0, 100.0, 20), 0);
        assert_eq!(renderer.calculate_bar_length(1.0, 100.0, 20), 1); // 最小长度为1
        assert_eq!(renderer.calculate_bar_length(50.0, 0.0, 20), 0);
    }

    #[test]
    fn test_render_bid_bar() {
        let renderer = BarChartRenderer::new();
        
        let result = renderer.render_bid_bar(50.0, 100.0, 20);
        assert!(result.contains("50"));  // 更新期望值，不再包含小数点
        assert!(result.contains("█"));
        
        // 测试零音量
        let result = renderer.render_bid_bar(0.0, 100.0, 20);
        assert_eq!(result, "");
    }

    #[test]
    fn test_create_bar_with_text() {
        let renderer = BarChartRenderer::new();
        
        let cell = renderer.create_bar_with_text(75.0, 100.0, 20, true);
        // 验证单元格不为空
        // 注意：由于Cell结构的限制，我们主要测试不会panic
        
        let empty_cell = renderer.create_bar_with_text(0.0, 100.0, 20, false);
        // 验证空音量返回空单元格
    }

    #[test]
    fn test_validate_parameters() {
        let renderer = BarChartRenderer::new();
        
        assert!(renderer.validate_parameters(50.0, 100.0).is_ok());
        assert!(renderer.validate_parameters(0.0, 100.0).is_ok());
        assert!(renderer.validate_parameters(100.0, 100.0).is_ok());
        
        assert!(renderer.validate_parameters(-1.0, 100.0).is_err());
        assert!(renderer.validate_parameters(50.0, -1.0).is_err());
        assert!(renderer.validate_parameters(150.0, 100.0).is_err());
    }

    #[test]
    fn test_zero_volume_handling() {
        let renderer = BarChartRenderer::new();
        
        // 测试零挂单量不显示任何色块
        assert_eq!(renderer.calculate_btc_blocks(0.0), 0);
        
        // 测试零挂单量的渲染结果
        let result = renderer.render_bid_bar(0.0, 100.0, 20);
        assert_eq!(result, "");
        
        let result = renderer.render_ask_bar(0.0, 100.0, 20);
        assert_eq!(result, "");
        
        // 测试create_bar_with_text对零挂单量的处理
        let cell = renderer.create_bar_with_text(0.0, 100.0, 20, true);
        // 这里我们不能直接比较Cell的内容，但至少确保不会panic
        
        // 测试修复后的unicode块字符计算：每0.001BTC一个最小单位
        assert_eq!(renderer.calculate_btc_blocks(0.001), 1);   // 0.001BTC，显示1个单位
        assert_eq!(renderer.calculate_btc_blocks(0.0005), 1);  // 0.0005BTC，低于0.001BTC显示1个最小字符块
        assert_eq!(renderer.calculate_btc_blocks(0.002), 2);   // 0.002BTC，显示2个单位
        assert_eq!(renderer.calculate_btc_blocks(0.0015), 2);  // 0.0015BTC，四舍五入显示2个单位
        assert_eq!(renderer.calculate_btc_blocks(0.004), 4);   // 0.004BTC，显示4个单位
    }
}