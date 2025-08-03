import * as stores from '../index';

describe('Stores Index', () => {
  it('should export all store hooks', () => {
    expect(stores.useOrderBookStore).toBeDefined();
    expect(stores.useConnectionStore).toBeDefined();
    expect(stores.useSettingsStore).toBeDefined();
  });

  it('should export orderbook utility functions', () => {
    expect(stores.getOrderBookLevels).toBeDefined();
    expect(stores.getBidLevels).toBeDefined();
    expect(stores.getAskLevels).toBeDefined();
    expect(stores.getSpreadInfo).toBeDefined();
    expect(stores.getTotalVolume).toBeDefined();
  });

  it('should export connection utility functions', () => {
    expect(stores.getConnectionHealth).toBeDefined();
    expect(stores.getReconnectionInfo).toBeDefined();
    expect(stores.getStreamInfo).toBeDefined();
    expect(stores.formatConnectionStatus).toBeDefined();
    expect(stores.formatLatency).toBeDefined();
    expect(stores.formatUptime).toBeDefined();
  });

  it('should export settings utility functions', () => {
    expect(stores.getThemePreference).toBeDefined();
    expect(stores.applyThemeToDocument).toBeDefined();
    expect(stores.applyColorSchemeToDocument).toBeDefined();
    expect(stores.validatePriceStep).toBeDefined();
    expect(stores.validateMaxLevels).toBeDefined();
    expect(stores.validateTimeWindow).toBeDefined();
    expect(stores.validateMaxTradeHistory).toBeDefined();
    expect(stores.validateRenderThrottle).toBeDefined();
    expect(stores.formatTimeWindow).toBeDefined();
    expect(stores.formatRenderRate).toBeDefined();
    expect(stores.migrateSettings).toBeDefined();
  });
});