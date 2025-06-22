#!/usr/bin/env python3
"""
Create a simple logo using basic Python without external dependencies.
This creates a simple text-based logo that can be used as a placeholder.
"""

def create_simple_logo_data():
    """
    Create a simple 64x64 RGBA logo data.
    This creates a basic geometric pattern that represents trading/finance.
    """
    width, height = 64, 64
    
    # Create RGBA data (4 bytes per pixel)
    data = []
    
    for y in range(height):
        for x in range(width):
            # Create a circular background
            center_x, center_y = width // 2, height // 2
            distance = ((x - center_x) ** 2 + (y - center_y) ** 2) ** 0.5
            
            if distance <= 30:  # Inside circle
                # Create a gradient effect
                intensity = int(255 * (1 - distance / 30))
                
                # Create some pattern for trading theme
                if (x + y) % 8 < 4:  # Checkerboard-like pattern
                    r, g, b = min(255, intensity + 50), min(255, intensity + 100), 255
                else:
                    r, g, b = 50, min(255, intensity + 50), min(255, intensity + 100)
                
                a = min(255, intensity + 100)  # Alpha
            else:
                r, g, b, a = 0, 0, 0, 0  # Transparent outside
            
            data.extend([r, g, b, a])
    
    return width, height, bytes(data)

def save_as_ppm(filename, width, height, rgb_data):
    """Save as PPM format (simple format that can be converted)"""
    with open(filename, 'wb') as f:
        # PPM header
        f.write(f"P6\n{width} {height}\n255\n".encode())
        
        # Convert RGBA to RGB
        rgb_only = []
        for i in range(0, len(rgb_data), 4):
            rgb_only.extend(rgb_data[i:i+3])  # Skip alpha channel
        
        f.write(bytes(rgb_only))

def main():
    print("Creating simple logo...")
    
    width, height, rgba_data = create_simple_logo_data()
    
    # Save as PPM (simple format)
    ppm_path = 'src/image/logo.ppm'
    save_as_ppm(ppm_path, width, height, rgba_data)
    print(f"Simple logo saved as {ppm_path}")
    
    print("Note: This creates a PPM file. For PNG support, you would need PIL/Pillow.")
    print("The application will fall back to text logo if PNG is not found.")

if __name__ == "__main__":
    main()
