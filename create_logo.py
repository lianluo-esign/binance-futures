#!/usr/bin/env python3
"""
Simple script to create a demo logo for the binance-futures application.
This creates a 64x64 PNG logo with a trading chart icon.
"""

from PIL import Image, ImageDraw, ImageFont
import os

def create_logo():
    # Create a 64x64 image with transparent background
    size = (64, 64)
    img = Image.new('RGBA', size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Draw a simple trading chart icon
    # Background circle
    draw.ellipse([4, 4, 60, 60], fill=(30, 50, 100, 200), outline=(100, 150, 255, 255), width=2)
    
    # Draw candlestick chart lines
    # Upward trend line
    draw.line([15, 45, 25, 35, 35, 30, 45, 20], fill=(0, 255, 100, 255), width=2)
    
    # Draw some candlesticks
    # Green candle
    draw.rectangle([18, 25, 22, 35], fill=(0, 255, 100, 255))
    draw.line([20, 23, 20, 37], fill=(0, 255, 100, 255), width=1)
    
    # Red candle
    draw.rectangle([28, 30, 32, 40], fill=(255, 100, 100, 255))
    draw.line([30, 28, 30, 42], fill=(255, 100, 100, 255), width=1)
    
    # Green candle
    draw.rectangle([38, 20, 42, 30], fill=(0, 255, 100, 255))
    draw.line([40, 18, 40, 32], fill=(0, 255, 100, 255), width=1)
    
    # Add "BF" text (Binance Futures)
    try:
        # Try to use a default font
        font = ImageFont.load_default()
        draw.text((24, 48), "BF", fill=(255, 255, 255, 255), font=font)
    except:
        # Fallback if font loading fails
        draw.text((24, 48), "BF", fill=(255, 255, 255, 255))
    
    return img

def main():
    # Create the logo
    logo = create_logo()
    
    # Ensure the directory exists
    os.makedirs('src/image', exist_ok=True)
    
    # Save the logo
    logo_path = 'src/image/logo.png'
    logo.save(logo_path, 'PNG')
    print(f"Logo created and saved to {logo_path}")
    
    # Also create a larger version for better quality
    large_logo = create_logo().resize((128, 128), Image.Resampling.LANCZOS)
    large_logo_path = 'src/image/logo_large.png'
    large_logo.save(large_logo_path, 'PNG')
    print(f"Large logo created and saved to {large_logo_path}")

if __name__ == "__main__":
    main()
