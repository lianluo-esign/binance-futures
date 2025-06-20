// UI相关的辅助函数和组件
// 目前UI逻辑在main.rs中实现，这个文件为将来的UI模块化预留

pub struct UIState {
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub selected_tab: usize,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            scroll_offset: 0,
            auto_scroll: true,
            selected_tab: 0,
        }
    }
}

impl UIState {
    pub fn new() -> Self {
        Self::default()
    }
}
