use crate::ReactiveApp;
use crate::core::{ProviderType};
use crate::core::provider::ProviderMetrics;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Paragraph},
    text::{Line, Span},
};

/// Status bar renderer for displaying system status information
pub struct StatusBar;

impl StatusBar {
    /// Creates a new StatusBar instance
    pub fn new() -> Self {
        Self
    }

    /// Renders the status bar with system information
    pub fn render(f: &mut Frame, app: &ReactiveApp, area: Rect) {
        // Get application statistics
        let stats = app.get_stats();
        
        // Get buffer usage information
        let (current_buffer_size, max_buffer_capacity) = app.get_buffer_usage();
        
        // Get Provider status information
        let provider_status = app.get_provider_status();
        
        // Build status display based on Provider type
        let status_spans = if let Some(ref status) = provider_status {
            match &status.provider_type {
                ProviderType::Binance { mode: _ } => {
                    // Binance real-time data Provider - show WebSocket connection status
                    let connection_status = if status.is_connected {
                        "Connected"
                    } else {
                        "Disconnected"
                    };
                    
                    vec![
                        Span::styled("Provider: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled("Binance WebSocket", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(" | Symbol: ", Style::default().fg(Color::White)),
                        Span::styled(app.get_symbol(), Style::default().fg(Color::Yellow)),
                        Span::styled(" | Status: ", Style::default().fg(Color::White)),
                        Span::styled(connection_status, if status.is_connected { 
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) 
                        } else { 
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD) 
                        }),
                        Span::styled(" | Buffer: ", Style::default().fg(Color::White)),
                        Span::styled(format!("{}/{}", current_buffer_size, max_buffer_capacity), Style::default().fg(Color::Yellow)),
                        Span::styled(" | Events/s: ", Style::default().fg(Color::White)),
                        Span::styled(format!("{:.1}", stats.events_processed_per_second), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                    ]
                },
                ProviderType::HistoricalData { format: _ } => {
                    // Historical data Provider - show playback progress and file info
                    let progress = if let Some(ref metrics) = status.metrics {
                        match metrics {
                            ProviderMetrics::Historical { 
                                file_progress, 
                                playback_speed: _, 
                                processed_events: _, 
                                total_events: _,
                                .. 
                            } => {
                                format!("{:.1}%", file_progress * 100.0)
                            },
                            _ => "0.0%".to_string()
                        }
                    } else {
                        "0.0%".to_string()
                    };
                    
                    let playback_speed = if let Some(ref metrics) = status.metrics {
                        match metrics {
                            ProviderMetrics::Historical { 
                                playback_speed, 
                                .. 
                            } => format!("{:.1}x", playback_speed),
                            _ => "1.0x".to_string()
                        }
                    } else {
                        "1.0x".to_string()
                    };
                    
                    let events_info = if let Some(ref metrics) = status.metrics {
                        match metrics {
                            ProviderMetrics::Historical { 
                                processed_events, 
                                total_events,
                                .. 
                            } => format!("{}/{}", processed_events, total_events),
                            _ => "0/0".to_string()
                        }
                    } else {
                        "0/0".to_string()
                    };
                    
                    let file_info = if let Some(ref metadata) = status.custom_metadata {
                        if let Some(file_path) = metadata.get("file_path") {
                            if let Some(path_str) = file_path.as_str() {
                                // Only show filename, not full path
                                std::path::Path::new(path_str)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown.gz")
                                    .to_string()
                            } else {
                                "unknown.gz".to_string()
                            }
                        } else {
                            "unknown.gz".to_string()
                        }
                    } else {
                        "unknown.gz".to_string()
                    };
                    
                    vec![
                        Span::styled("Provider: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled("Historical Data", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(" | File: ", Style::default().fg(Color::White)),
                        Span::styled(file_info, Style::default().fg(Color::Yellow)),
                        Span::styled(" | Progress: ", Style::default().fg(Color::White)),
                        Span::styled(progress, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled(" | Speed: ", Style::default().fg(Color::White)),
                        Span::styled(playback_speed, Style::default().fg(Color::Magenta)),
                        Span::styled(" | Events: ", Style::default().fg(Color::White)),
                        Span::styled(events_info, Style::default().fg(Color::Blue)),
                        Span::styled(" | Buffer: ", Style::default().fg(Color::White)),
                        Span::styled(format!("{}/{}", current_buffer_size, max_buffer_capacity), Style::default().fg(Color::Yellow)),
                    ]
                },
                _ => {
                    // Other Provider types - show generic information
                    vec![
                        Span::styled("Provider: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled("Unknown", Style::default().fg(Color::Gray)),
                        Span::styled(" | Symbol: ", Style::default().fg(Color::White)),
                        Span::styled(app.get_symbol(), Style::default().fg(Color::Cyan)),
                        Span::styled(" | Buffer: ", Style::default().fg(Color::White)),
                        Span::styled(format!("{}/{}", current_buffer_size, max_buffer_capacity), Style::default().fg(Color::Yellow)),
                        Span::styled(" | Events/s: ", Style::default().fg(Color::White)),
                        Span::styled(format!("{:.1}", stats.events_processed_per_second), Style::default().fg(Color::Magenta)),
                    ]
                }
            }
        } else {
            // Fallback display when no Provider status info (legacy system)
            let connection_status = if stats.websocket_connected {
                "Connected"
            } else {
                "Disconnected"
            };
            
            vec![
                Span::styled("Symbol: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(app.get_symbol(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" | Buffer: ", Style::default().fg(Color::White)),
                Span::styled(format!("{}/{}", current_buffer_size, max_buffer_capacity), Style::default().fg(Color::Yellow)),
                Span::styled(" | Status: ", Style::default().fg(Color::White)),
                Span::styled(connection_status, if stats.websocket_connected { 
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) 
                } else { 
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD) 
                }),
                Span::styled(" | Events/s: ", Style::default().fg(Color::White)),
                Span::styled(format!("{:.1}", stats.events_processed_per_second), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ]
        };
       
        // Create status information text (single line display)
        let status_text = vec![Line::from(status_spans)];
        
        // Create status bar paragraph
        let status_paragraph = Paragraph::new(status_text)
            .block(Block::default()
                .title("System Status")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
            );
        
        f.render_widget(status_paragraph, area);
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}