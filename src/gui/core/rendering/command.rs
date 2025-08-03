/// 渲染命令定义

use eframe::egui;
use serde::{Deserialize, Serialize};

use super::super::component::ComponentId;

/// 渲染目标
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderTarget {
    /// 主窗口
    MainWindow,
    /// 子窗口
    ChildWindow(String),
    /// 离屏渲染缓冲区
    OffscreenBuffer(String),
    /// 纹理目标
    Texture(String),
}

/// 渲染命令
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// 清空渲染目标
    Clear {
        target: RenderTarget,
        color: egui::Color32,
    },
    /// 渲染组件
    RenderComponent {
        component_id: ComponentId,
        target: RenderTarget,
        viewport: egui::Rect,
        z_index: i32,
    },
    /// 渲染文本
    RenderText {
        text: String,
        position: egui::Pos2,
        color: egui::Color32,
        font: egui::FontId,
        target: RenderTarget,
    },
    /// 渲染矩形
    RenderRect {
        rect: egui::Rect,
        fill: egui::Color32,
        stroke: egui::Stroke,
        target: RenderTarget,
    },
    /// 渲染线条
    RenderLine {
        points: Vec<egui::Pos2>,
        stroke: egui::Stroke,
        target: RenderTarget,
    },
    /// 渲染图像
    RenderImage {
        texture_id: egui::TextureId,
        rect: egui::Rect,
        uv: egui::Rect,
        tint: egui::Color32,
        target: RenderTarget,
    },
    /// 设置剪裁区域
    SetClipRect {
        rect: egui::Rect,
        target: RenderTarget,
    },
    /// 应用变换矩阵
    ApplyTransform {
        transform: egui::emath::TSTransform,
        target: RenderTarget,
    },
    /// 开始渲染组
    BeginGroup {
        group_id: String,
        target: RenderTarget,
    },
    /// 结束渲染组
    EndGroup {
        group_id: String,
        target: RenderTarget,
    },
    /// 自定义渲染命令
    Custom {
        command_type: String,
        data: serde_json::Value,
        target: RenderTarget,
    },
}

impl RenderCommand {
    /// 获取渲染命令的目标
    pub fn target(&self) -> &RenderTarget {
        match self {
            RenderCommand::Clear { target, .. } => target,
            RenderCommand::RenderComponent { target, .. } => target,
            RenderCommand::RenderText { target, .. } => target,
            RenderCommand::RenderRect { target, .. } => target,
            RenderCommand::RenderLine { target, .. } => target,
            RenderCommand::RenderImage { target, .. } => target,
            RenderCommand::SetClipRect { target, .. } => target,
            RenderCommand::ApplyTransform { target, .. } => target,
            RenderCommand::BeginGroup { target, .. } => target,
            RenderCommand::EndGroup { target, .. } => target,
            RenderCommand::Custom { target, .. } => target,
        }
    }
    
    /// 获取渲染命令的z-index (用于排序)
    pub fn z_index(&self) -> i32 {
        match self {
            RenderCommand::RenderComponent { z_index, .. } => *z_index,
            _ => 0,
        }
    }
}