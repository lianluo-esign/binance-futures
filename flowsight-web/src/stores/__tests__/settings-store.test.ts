import { act, renderHook } from '@testing-library/react';
import {
  useSettingsStore,
  getThemePreference,
  validatePriceStep,
  validateMaxLevels,
  validateTimeWindow,
  validateMaxTradeHistory,
  validateRenderThrottle,
  formatTimeWindow,
  formatRenderRate,
  migrateSettings,
} from '../settings-store';

// Mock localStorage
const localStorageMock = {
  getItem: jest.fn(),
  setItem: jest.fn(),
  removeItem: jest.fn(),
  clear: jest.fn(),
};

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
});

// Mock matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: jest.fn().mockImplementation(query => ({
    matches: query.includes('dark'),
    media: query,
    onchange: null,
    addListener: jest.fn(),
    removeListener: jest.fn(),
    addEventListener: jest.fn(),
    removeEventListener: jest.fn(),
    dispatchEvent: jest.fn(),
  })),
});

// Mock document
Object.defineProperty(document, 'documentElement', {
  value: {
    classList: {
      add: jest.fn(),
      remove: jest.fn(),
      toggle: jest.fn(),
    },
  },
});

describe('Settings Store', () => {
  beforeEach(() => {
    // Clear localStorage mock
    localStorageMock.getItem.mockClear();
    localStorageMock.setItem.mockClear();
    localStorageMock.removeItem.mockClear();
    localStorageMock.clear.mockClear();
    
    // Reset document class list mocks
    (document.documentElement.classList.add as jest.Mock).mockClear();
    (document.documentElement.classList.remove as jest.Mock).mockClear();
    (document.documentElement.classList.toggle as jest.Mock).mockClear();
  });

  describe('Initial State', () => {
    it('should have correct default values', () => {
      const { result } = renderHook(() => useSettingsStore());
      const state = result.current;

      expect(state.theme).toBe('dark');
      expect(state.colorScheme).toBe('default');
      expect(state.defaultSymbol).toBe('BTCUSDT');
      expect(state.defaultPriceStep).toBe(0.5);
      expect(state.maxOrderBookLevels).toBe(40);
      expect(state.chartTimeWindow).toBe(5 * 60 * 1000);
      expect(state.showVolumeDots).toBe(true);
      expect(state.enableAnimations).toBe(true);
      expect(state.showGridLines).toBe(true);
      expect(state.autoScale).toBe(true);
      expect(state.maxTradeHistory).toBe(1000);
      expect(state.dataCleanupInterval).toBe(5 * 60 * 1000);
      expect(state.renderThrottleMs).toBe(16);
      expect(state.exportFormat).toBe('json');
      expect(state.includeTimestamps).toBe(true);
    });
  });

  describe('Theme Management', () => {
    it('should set theme and apply to document', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setTheme('light');
      });

      expect(result.current.theme).toBe('light');
      expect(document.documentElement.classList.toggle).toHaveBeenCalledWith('dark', false);
    });

    it('should handle auto theme', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setTheme('auto');
      });

      expect(result.current.theme).toBe('auto');
      expect(document.documentElement.classList.toggle).toHaveBeenCalledWith('dark', true); // matchMedia mock returns true for dark
    });
  });

  describe('Color Scheme Management', () => {
    it('should set color scheme and apply to document', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setColorScheme('high-contrast');
      });

      expect(result.current.colorScheme).toBe('high-contrast');
      expect(document.documentElement.classList.remove).toHaveBeenCalledWith('high-contrast', 'colorblind');
      expect(document.documentElement.classList.add).toHaveBeenCalledWith('high-contrast');
    });

    it('should remove color scheme classes for default', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setColorScheme('colorblind');
      });

      act(() => {
        result.current.setColorScheme('default');
      });

      expect(result.current.colorScheme).toBe('default');
      expect(document.documentElement.classList.remove).toHaveBeenCalledWith('high-contrast', 'colorblind');
    });
  });

  describe('Trading Settings', () => {
    it('should set default symbol in uppercase', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setDefaultSymbol('ethusdt');
      });

      expect(result.current.defaultSymbol).toBe('ETHUSDT');
    });

    it('should set valid price step', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setDefaultPriceStep(1.0);
      });

      expect(result.current.defaultPriceStep).toBe(1.0);
    });

    it('should reject invalid price step', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initialStep = result.current.defaultPriceStep;

      act(() => {
        result.current.setDefaultPriceStep(-1.0);
      });

      expect(result.current.defaultPriceStep).toBe(initialStep);
    });

    it('should set valid max order book levels', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setMaxOrderBookLevels(50);
      });

      expect(result.current.maxOrderBookLevels).toBe(50);
    });

    it('should reject invalid max order book levels', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initialLevels = result.current.maxOrderBookLevels;

      act(() => {
        result.current.setMaxOrderBookLevels(0);
      });

      expect(result.current.maxOrderBookLevels).toBe(initialLevels);

      act(() => {
        result.current.setMaxOrderBookLevels(150);
      });

      expect(result.current.maxOrderBookLevels).toBe(initialLevels);
    });

    it('should set valid chart time window', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setChartTimeWindow(10 * 60 * 1000);
      });

      expect(result.current.chartTimeWindow).toBe(10 * 60 * 1000);
    });
  });

  describe('Chart Settings', () => {
    it('should toggle volume dots', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initial = result.current.showVolumeDots;

      act(() => {
        result.current.toggleVolumeDots();
      });

      expect(result.current.showVolumeDots).toBe(!initial);
    });

    it('should toggle animations and apply to document', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initial = result.current.enableAnimations;

      act(() => {
        result.current.toggleAnimations();
      });

      expect(result.current.enableAnimations).toBe(!initial);
      expect(document.documentElement.classList.toggle).toHaveBeenCalledWith('reduce-motion', initial);
    });

    it('should toggle grid lines', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initial = result.current.showGridLines;

      act(() => {
        result.current.toggleGridLines();
      });

      expect(result.current.showGridLines).toBe(!initial);
    });

    it('should toggle auto scale', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initial = result.current.autoScale;

      act(() => {
        result.current.toggleAutoScale();
      });

      expect(result.current.autoScale).toBe(!initial);
    });
  });

  describe('Performance Settings', () => {
    it('should set valid max trade history', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setMaxTradeHistory(2000);
      });

      expect(result.current.maxTradeHistory).toBe(2000);
    });

    it('should reject invalid max trade history', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initial = result.current.maxTradeHistory;

      act(() => {
        result.current.setMaxTradeHistory(0);
      });

      expect(result.current.maxTradeHistory).toBe(initial);

      act(() => {
        result.current.setMaxTradeHistory(15000);
      });

      expect(result.current.maxTradeHistory).toBe(initial);
    });

    it('should set valid data cleanup interval', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setDataCleanupInterval(10 * 60 * 1000);
      });

      expect(result.current.dataCleanupInterval).toBe(10 * 60 * 1000);
    });

    it('should set valid render throttle', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setRenderThrottleMs(33);
      });

      expect(result.current.renderThrottleMs).toBe(33);
    });

    it('should reject invalid render throttle', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initial = result.current.renderThrottleMs;

      act(() => {
        result.current.setRenderThrottleMs(5);
      });

      expect(result.current.renderThrottleMs).toBe(initial);

      act(() => {
        result.current.setRenderThrottleMs(150);
      });

      expect(result.current.renderThrottleMs).toBe(initial);
    });
  });

  describe('Export Settings', () => {
    it('should set export format', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.setExportFormat('csv');
      });

      expect(result.current.exportFormat).toBe('csv');
    });

    it('should toggle include timestamps', () => {
      const { result } = renderHook(() => useSettingsStore());
      const initial = result.current.includeTimestamps;

      act(() => {
        result.current.toggleIncludeTimestamps();
      });

      expect(result.current.includeTimestamps).toBe(!initial);
    });
  });

  describe('Reset and Storage', () => {
    it('should reset to defaults', () => {
      const { result } = renderHook(() => useSettingsStore());

      // Change some settings
      act(() => {
        result.current.setTheme('light');
        result.current.setDefaultSymbol('ETHUSDT');
        result.current.toggleVolumeDots();
      });

      // Reset
      act(() => {
        result.current.resetToDefaults();
      });

      const state = result.current;
      expect(state.theme).toBe('dark');
      expect(state.defaultSymbol).toBe('BTCUSDT');
      expect(state.showVolumeDots).toBe(true);
    });

    it('should load from storage and apply settings', () => {
      const { result } = renderHook(() => useSettingsStore());

      act(() => {
        result.current.loadFromStorage();
      });

      // Should apply theme and color scheme to document
      expect(document.documentElement.classList.toggle).toHaveBeenCalled();
    });
  });
});

describe('Settings Utility Functions', () => {
  describe('Validation Functions', () => {
    it('should validate price step', () => {
      expect(validatePriceStep(0.5)).toBe(true);
      expect(validatePriceStep(1.0)).toBe(true);
      expect(validatePriceStep(0)).toBe(false);
      expect(validatePriceStep(-1)).toBe(false);
      expect(validatePriceStep(150)).toBe(false);
      expect(validatePriceStep(Infinity)).toBe(false);
      expect(validatePriceStep(NaN)).toBe(false);
    });

    it('should validate max levels', () => {
      expect(validateMaxLevels(40)).toBe(true);
      expect(validateMaxLevels(100)).toBe(true);
      expect(validateMaxLevels(0)).toBe(false);
      expect(validateMaxLevels(-1)).toBe(false);
      expect(validateMaxLevels(150)).toBe(false);
      expect(validateMaxLevels(40.5)).toBe(false);
    });

    it('should validate time window', () => {
      expect(validateTimeWindow(5 * 60 * 1000)).toBe(true);
      expect(validateTimeWindow(24 * 60 * 60 * 1000)).toBe(true);
      expect(validateTimeWindow(5.5 * 60 * 1000)).toBe(true); // Allow non-integer values
      expect(validateTimeWindow(0)).toBe(false);
      expect(validateTimeWindow(-1000)).toBe(false);
      expect(validateTimeWindow(25 * 60 * 60 * 1000)).toBe(false);
    });

    it('should validate max trade history', () => {
      expect(validateMaxTradeHistory(1000)).toBe(true);
      expect(validateMaxTradeHistory(10000)).toBe(true);
      expect(validateMaxTradeHistory(0)).toBe(false);
      expect(validateMaxTradeHistory(-1)).toBe(false);
      expect(validateMaxTradeHistory(15000)).toBe(false);
      expect(validateMaxTradeHistory(1000.5)).toBe(false);
    });

    it('should validate render throttle', () => {
      expect(validateRenderThrottle(16)).toBe(true);
      expect(validateRenderThrottle(33)).toBe(true);
      expect(validateRenderThrottle(100)).toBe(true);
      expect(validateRenderThrottle(7)).toBe(false);
      expect(validateRenderThrottle(150)).toBe(false);
      expect(validateRenderThrottle(16.5)).toBe(false);
    });
  });

  describe('Formatting Functions', () => {
    it('should format time window', () => {
      expect(formatTimeWindow(30 * 1000)).toBe('30s');
      expect(formatTimeWindow(90 * 1000)).toBe('1m 30s');
      expect(formatTimeWindow(5 * 60 * 1000)).toBe('5m 0s');
      expect(formatTimeWindow(90 * 60 * 1000)).toBe('1h 30m');
      expect(formatTimeWindow(2 * 60 * 60 * 1000)).toBe('2h 0m');
    });

    it('should format render rate', () => {
      expect(formatRenderRate(16)).toBe('63 FPS');
      expect(formatRenderRate(33)).toBe('30 FPS');
      expect(formatRenderRate(100)).toBe('10 FPS');
    });
  });

  describe('Settings Migration', () => {
    it('should migrate valid settings', () => {
      const stored = {
        theme: 'light',
        colorScheme: 'high-contrast',
        defaultSymbol: 'ethusdt',
        defaultPriceStep: 1.0,
        maxOrderBookLevels: 50,
        chartTimeWindow: 10 * 60 * 1000,
        showVolumeDots: false,
        enableAnimations: false,
        showGridLines: false,
        autoScale: false,
        maxTradeHistory: 2000,
        dataCleanupInterval: 10 * 60 * 1000,
        renderThrottleMs: 33,
        exportFormat: 'csv',
        includeTimestamps: false,
      };

      const migrated = migrateSettings(stored);

      expect(migrated.theme).toBe('light');
      expect(migrated.colorScheme).toBe('high-contrast');
      expect(migrated.defaultSymbol).toBe('ETHUSDT');
      expect(migrated.defaultPriceStep).toBe(1.0);
      expect(migrated.maxOrderBookLevels).toBe(50);
      expect(migrated.chartTimeWindow).toBe(10 * 60 * 1000);
      expect(migrated.showVolumeDots).toBe(false);
      expect(migrated.enableAnimations).toBe(false);
      expect(migrated.showGridLines).toBe(false);
      expect(migrated.autoScale).toBe(false);
      expect(migrated.maxTradeHistory).toBe(2000);
      expect(migrated.dataCleanupInterval).toBe(10 * 60 * 1000);
      expect(migrated.renderThrottleMs).toBe(33);
      expect(migrated.exportFormat).toBe('csv');
      expect(migrated.includeTimestamps).toBe(false);
    });

    it('should reject invalid settings during migration', () => {
      const stored = {
        theme: 'invalid',
        colorScheme: 'invalid',
        defaultSymbol: 123,
        defaultPriceStep: -1,
        maxOrderBookLevels: 150,
        chartTimeWindow: -1000,
        showVolumeDots: 'invalid',
        maxTradeHistory: 15000,
        renderThrottleMs: 5,
        exportFormat: 'invalid',
      };

      const migrated = migrateSettings(stored);

      expect(migrated.theme).toBeUndefined();
      expect(migrated.colorScheme).toBeUndefined();
      expect(migrated.defaultSymbol).toBeUndefined();
      expect(migrated.defaultPriceStep).toBeUndefined();
      expect(migrated.maxOrderBookLevels).toBeUndefined();
      expect(migrated.chartTimeWindow).toBeUndefined();
      expect(migrated.showVolumeDots).toBeUndefined();
      expect(migrated.maxTradeHistory).toBeUndefined();
      expect(migrated.renderThrottleMs).toBeUndefined();
      expect(migrated.exportFormat).toBeUndefined();
    });
  });

  describe('getThemePreference', () => {
    it('should return stored theme preference', () => {
      localStorageMock.getItem.mockReturnValue(
        JSON.stringify({ state: { theme: 'light' } })
      );

      const theme = getThemePreference();
      expect(theme).toBe('light');
    });

    it('should handle auto theme preference', () => {
      localStorageMock.getItem.mockReturnValue(
        JSON.stringify({ state: { theme: 'auto' } })
      );

      const theme = getThemePreference();
      expect(theme).toBe('dark'); // matchMedia mock returns true for dark
    });

    it('should fallback to system preference when no stored value', () => {
      localStorageMock.getItem.mockReturnValue(null);

      const theme = getThemePreference();
      expect(theme).toBe('dark'); // matchMedia mock returns true for dark
    });

    it('should handle invalid stored JSON', () => {
      localStorageMock.getItem.mockReturnValue('invalid json');

      const theme = getThemePreference();
      expect(theme).toBe('dark'); // fallback to system preference
    });
  });
});