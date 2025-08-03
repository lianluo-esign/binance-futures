import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { SettingsState } from '../types';

// Default values
const DEFAULT_SYMBOL = 'BTCUSDT';
const DEFAULT_PRICE_STEP = 0.5;
const DEFAULT_MAX_ORDERBOOK_LEVELS = 40;
const DEFAULT_CHART_TIME_WINDOW = 5 * 60 * 1000; // 5 minutes
const DEFAULT_MAX_TRADE_HISTORY = 1000;
const DEFAULT_DATA_CLEANUP_INTERVAL = 5 * 60 * 1000; // 5 minutes
const DEFAULT_RENDER_THROTTLE_MS = 16; // ~60fps

const defaultSettings: Omit<SettingsState, keyof SettingsActions> = {
  // Display settings
  theme: 'dark',
  colorScheme: 'default',

  // Trading settings
  defaultSymbol: DEFAULT_SYMBOL,
  defaultPriceStep: DEFAULT_PRICE_STEP,
  maxOrderBookLevels: DEFAULT_MAX_ORDERBOOK_LEVELS,
  chartTimeWindow: DEFAULT_CHART_TIME_WINDOW,

  // Chart settings
  showVolumeDots: true,
  enableAnimations: true,
  showGridLines: true,
  autoScale: true,

  // Performance settings
  maxTradeHistory: DEFAULT_MAX_TRADE_HISTORY,
  dataCleanupInterval: DEFAULT_DATA_CLEANUP_INTERVAL,
  renderThrottleMs: DEFAULT_RENDER_THROTTLE_MS,

  // Export settings
  exportFormat: 'json',
  includeTimestamps: true,
};

interface SettingsActions {
  setTheme: (theme: 'light' | 'dark' | 'auto') => void;
  setColorScheme: (scheme: 'default' | 'high-contrast' | 'colorblind') => void;
  setDefaultSymbol: (symbol: string) => void;
  setDefaultPriceStep: (step: number) => void;
  setMaxOrderBookLevels: (levels: number) => void;
  setChartTimeWindow: (window: number) => void;
  toggleVolumeDots: () => void;
  toggleAnimations: () => void;
  toggleGridLines: () => void;
  toggleAutoScale: () => void;
  setMaxTradeHistory: (max: number) => void;
  setDataCleanupInterval: (interval: number) => void;
  setRenderThrottleMs: (ms: number) => void;
  setExportFormat: (format: 'json' | 'csv') => void;
  toggleIncludeTimestamps: () => void;
  resetToDefaults: () => void;
  loadFromStorage: () => void;
  saveToStorage: () => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set, get) => ({
      ...defaultSettings,

      // Actions
      setTheme: (theme: 'light' | 'dark' | 'auto') => {
        set({ theme });
        
        // Apply theme to document
        if (typeof window !== 'undefined') {
          const root = document.documentElement;
          if (theme === 'auto') {
            const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
            root.classList.toggle('dark', prefersDark);
          } else {
            root.classList.toggle('dark', theme === 'dark');
          }
        }
      },

      setColorScheme: (scheme: 'default' | 'high-contrast' | 'colorblind') => {
        set({ colorScheme: scheme });
        
        // Apply color scheme to document
        if (typeof window !== 'undefined') {
          const root = document.documentElement;
          root.classList.remove('high-contrast', 'colorblind');
          if (scheme !== 'default') {
            root.classList.add(scheme);
          }
        }
      },

      setDefaultSymbol: (symbol: string) => {
        set({ defaultSymbol: symbol.toUpperCase() });
      },

      setDefaultPriceStep: (step: number) => {
        if (step > 0) {
          set({ defaultPriceStep: step });
        }
      },

      setMaxOrderBookLevels: (levels: number) => {
        if (levels > 0 && levels <= 100) {
          set({ maxOrderBookLevels: levels });
        }
      },

      setChartTimeWindow: (window: number) => {
        if (window > 0) {
          set({ chartTimeWindow: window });
        }
      },

      toggleVolumeDots: () => {
        const state = get();
        set({ showVolumeDots: !state.showVolumeDots });
      },

      toggleAnimations: () => {
        const state = get();
        const newAnimationsState = !state.enableAnimations;
        set({ enableAnimations: newAnimationsState });
        
        // Apply animation preference to document
        if (typeof window !== 'undefined') {
          const root = document.documentElement;
          root.classList.toggle('reduce-motion', !newAnimationsState);
        }
      },

      toggleGridLines: () => {
        const state = get();
        set({ showGridLines: !state.showGridLines });
      },

      toggleAutoScale: () => {
        const state = get();
        set({ autoScale: !state.autoScale });
      },

      setMaxTradeHistory: (max: number) => {
        if (max > 0 && max <= 10000) {
          set({ maxTradeHistory: max });
        }
      },

      setDataCleanupInterval: (interval: number) => {
        if (interval > 0) {
          set({ dataCleanupInterval: interval });
        }
      },

      setRenderThrottleMs: (ms: number) => {
        if (ms >= 8 && ms <= 100) { // Between ~10fps and 120fps
          set({ renderThrottleMs: ms });
        }
      },

      setExportFormat: (format: 'json' | 'csv') => {
        set({ exportFormat: format });
      },

      toggleIncludeTimestamps: () => {
        const state = get();
        set({ includeTimestamps: !state.includeTimestamps });
      },

      resetToDefaults: () => {
        set(defaultSettings);
        
        // Apply default theme and color scheme
        if (typeof window !== 'undefined') {
          const root = document.documentElement;
          root.classList.toggle('dark', defaultSettings.theme === 'dark');
          root.classList.remove('high-contrast', 'colorblind', 'reduce-motion');
        }
      },

      loadFromStorage: () => {
        // This is handled automatically by the persist middleware
        // But we can trigger theme/color scheme application
        const state = get();
        if (typeof window !== 'undefined') {
          const root = document.documentElement;
          
          // Apply theme
          if (state.theme === 'auto') {
            const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
            root.classList.toggle('dark', prefersDark);
          } else {
            root.classList.toggle('dark', state.theme === 'dark');
          }
          
          // Apply color scheme
          root.classList.remove('high-contrast', 'colorblind');
          if (state.colorScheme !== 'default') {
            root.classList.add(state.colorScheme);
          }
          
          // Apply animation preference
          root.classList.toggle('reduce-motion', !state.enableAnimations);
        }
      },

      saveToStorage: () => {
        // This is handled automatically by the persist middleware
        // This method is here for explicit save operations if needed
      },
    }),
    {
      name: 'flowsight-settings',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        // Only persist settings, not actions
        theme: state.theme,
        colorScheme: state.colorScheme,
        defaultSymbol: state.defaultSymbol,
        defaultPriceStep: state.defaultPriceStep,
        maxOrderBookLevels: state.maxOrderBookLevels,
        chartTimeWindow: state.chartTimeWindow,
        showVolumeDots: state.showVolumeDots,
        enableAnimations: state.enableAnimations,
        showGridLines: state.showGridLines,
        autoScale: state.autoScale,
        maxTradeHistory: state.maxTradeHistory,
        dataCleanupInterval: state.dataCleanupInterval,
        renderThrottleMs: state.renderThrottleMs,
        exportFormat: state.exportFormat,
        includeTimestamps: state.includeTimestamps,
      }),
    }
  )
);

// Utility functions for settings management
export const getThemePreference = (): 'light' | 'dark' => {
  if (typeof window === 'undefined') return 'dark';
  
  const stored = localStorage.getItem('flowsight-settings');
  if (stored) {
    try {
      const settings = JSON.parse(stored);
      if (settings.state?.theme === 'auto') {
        return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      }
      return settings.state?.theme || 'dark';
    } catch {
      // Fall through to default
    }
  }
  
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
};

export const applyThemeToDocument = (theme: 'light' | 'dark' | 'auto') => {
  if (typeof window === 'undefined') return;
  
  const root = document.documentElement;
  if (theme === 'auto') {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    root.classList.toggle('dark', prefersDark);
  } else {
    root.classList.toggle('dark', theme === 'dark');
  }
};

export const applyColorSchemeToDocument = (scheme: 'default' | 'high-contrast' | 'colorblind') => {
  if (typeof window === 'undefined') return;
  
  const root = document.documentElement;
  root.classList.remove('high-contrast', 'colorblind');
  if (scheme !== 'default') {
    root.classList.add(scheme);
  }
};

export const validatePriceStep = (step: number): boolean => {
  return step > 0 && step <= 100 && Number.isFinite(step);
};

export const validateMaxLevels = (levels: number): boolean => {
  return Number.isInteger(levels) && levels > 0 && levels <= 100;
};

export const validateTimeWindow = (window: number): boolean => {
  return window > 0 && window <= 24 * 60 * 60 * 1000; // Max 24 hours
};

export const validateMaxTradeHistory = (max: number): boolean => {
  return Number.isInteger(max) && max > 0 && max <= 10000;
};

export const validateRenderThrottle = (ms: number): boolean => {
  return Number.isInteger(ms) && ms >= 8 && ms <= 100;
};

export const formatTimeWindow = (ms: number): string => {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  } else {
    return `${seconds}s`;
  }
};

export const formatRenderRate = (ms: number): string => {
  const fps = Math.round(1000 / ms);
  return `${fps} FPS`;
};

// Settings validation and migration
export const migrateSettings = (stored: any): Partial<SettingsState> => {
  const migrated: Partial<SettingsState> = {};

  // Migrate theme
  if (stored.theme && ['light', 'dark', 'auto'].includes(stored.theme)) {
    migrated.theme = stored.theme;
  }

  // Migrate color scheme
  if (stored.colorScheme && ['default', 'high-contrast', 'colorblind'].includes(stored.colorScheme)) {
    migrated.colorScheme = stored.colorScheme;
  }

  // Migrate trading settings
  if (stored.defaultSymbol && typeof stored.defaultSymbol === 'string') {
    migrated.defaultSymbol = stored.defaultSymbol.toUpperCase();
  }

  if (stored.defaultPriceStep && validatePriceStep(stored.defaultPriceStep)) {
    migrated.defaultPriceStep = stored.defaultPriceStep;
  }

  if (stored.maxOrderBookLevels && validateMaxLevels(stored.maxOrderBookLevels)) {
    migrated.maxOrderBookLevels = stored.maxOrderBookLevels;
  }

  if (stored.chartTimeWindow && validateTimeWindow(stored.chartTimeWindow)) {
    migrated.chartTimeWindow = stored.chartTimeWindow;
  }

  // Migrate boolean settings
  if (typeof stored.showVolumeDots === 'boolean') {
    migrated.showVolumeDots = stored.showVolumeDots;
  }

  if (typeof stored.enableAnimations === 'boolean') {
    migrated.enableAnimations = stored.enableAnimations;
  }

  if (typeof stored.showGridLines === 'boolean') {
    migrated.showGridLines = stored.showGridLines;
  }

  if (typeof stored.autoScale === 'boolean') {
    migrated.autoScale = stored.autoScale;
  }

  // Migrate performance settings
  if (stored.maxTradeHistory && validateMaxTradeHistory(stored.maxTradeHistory)) {
    migrated.maxTradeHistory = stored.maxTradeHistory;
  }

  if (stored.dataCleanupInterval && stored.dataCleanupInterval > 0) {
    migrated.dataCleanupInterval = stored.dataCleanupInterval;
  }

  if (stored.renderThrottleMs && validateRenderThrottle(stored.renderThrottleMs)) {
    migrated.renderThrottleMs = stored.renderThrottleMs;
  }

  // Migrate export settings
  if (stored.exportFormat && ['json', 'csv'].includes(stored.exportFormat)) {
    migrated.exportFormat = stored.exportFormat;
  }

  if (typeof stored.includeTimestamps === 'boolean') {
    migrated.includeTimestamps = stored.includeTimestamps;
  }

  return migrated;
};
