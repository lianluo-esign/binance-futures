# How to Add a Custom Logo

## Quick Start

1. **Create or obtain a PNG logo file**
   - Size: 64x64 or 128x128 pixels recommended
   - Format: PNG with transparency
   - Name: `logo.png`

2. **Place the file in the correct location**
   ```
   src/image/logo.png
   ```

3. **Restart the application**
   - The logo will be automatically loaded and displayed in the header

## Current Status

✅ **Logo system implemented and working**
- Logo loading from `src/image/logo.png`
- Automatic fallback to custom-drawn logo if PNG not found
- Responsive sizing and positioning in header
- Maintains aspect ratio

✅ **Enhanced fallback logo**
- Custom-drawn circular logo with trading chart icon
- Blue gradient background with trend line
- Professional appearance when no PNG file is available

## Logo Design Guidelines

### Technical Requirements
- **Format**: PNG (Portable Network Graphics)
- **Size**: 64x64 to 128x128 pixels
- **Background**: Transparent (RGBA)
- **Color depth**: 32-bit (8-bit per channel + alpha)

### Visual Guidelines
- **Style**: Clean, modern, professional
- **Colors**: High contrast, preferably blue/green theme to match UI
- **Complexity**: Simple design that works well at small sizes
- **Theme**: Trading/finance related (charts, arrows, currency symbols)

## Example Logo Ideas

1. **Candlestick Chart**: Simple 3-4 candlestick representation
2. **Trend Arrow**: Upward trending arrow with chart line
3. **Currency Symbol**: Stylized Bitcoin (₿) or Dollar ($) symbol
4. **Graph Icon**: Simple line chart or bar chart
5. **Geometric**: Abstract geometric pattern in brand colors

## Testing Your Logo

1. Place your `logo.png` file in `src/image/`
2. Run the application: `cargo run`
3. Check the header area - your logo should appear next to the title
4. Verify the logo scales properly and looks good

## Troubleshooting

**Logo not appearing?**
- Check file name is exactly `logo.png`
- Verify file is in `src/image/` directory
- Ensure PNG format with proper headers
- Check file permissions

**Logo looks blurry?**
- Use higher resolution (128x128 instead of 64x64)
- Ensure PNG is not compressed too heavily
- Use vector-based design tools for crisp edges

**Logo too large/small?**
- The system automatically scales to fit header height
- Aspect ratio is preserved
- No manual sizing needed
