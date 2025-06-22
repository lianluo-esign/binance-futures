# Logo Directory

Place your logo.png file in this directory to display a custom logo in the application header.

## Logo Requirements

The logo should be:
- **Format**: PNG format with transparency support
- **Size**: Recommended 64x64 pixels or 128x128 pixels
- **Background**: Transparent background preferred for best integration
- **Colors**: High contrast colors for visibility on dark backgrounds
- **Style**: Simple, clean design that works well at small sizes

## Implementation Details

The logo integration includes:
- Automatic loading from `src/image/logo.png`
- Fallback to text-based logo if PNG file is not found
- Responsive sizing that maintains aspect ratio
- Positioned in the application header next to the title

## Creating a Logo

You can create a logo using any image editing software:

1. **Adobe Photoshop/GIMP**: Create a 64x64 or 128x128 canvas with transparent background
2. **Online tools**: Use Canva, Figma, or similar tools
3. **Icon libraries**: Download from IconFinder, Flaticon, or similar
4. **Custom design**: Create a simple geometric design representing trading/finance

## Example Logo Ideas

For a trading application, consider:
- Candlestick chart icon
- Trending arrow symbols
- Currency symbols (₿, $, €)
- Graph/chart representations
- Geometric patterns in blue/green colors

## Current Status

- ✅ Logo loading system implemented
- ✅ Fallback text logo working
- ⏳ Waiting for custom PNG logo file
- ✅ Responsive display in header
