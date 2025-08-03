#!/usr/bin/env python3
"""
Generate all required app icons from the volcanic SVG logo
Requires: pip install Pillow cairosvg
"""

import os
import sys
from pathlib import Path
import cairosvg
from PIL import Image, ImageDraw, ImageFilter
import tempfile

def create_rounded_icon(image, radius_percent=20):
    """Create a rounded corner version of the image for modern app icons"""
    size = image.size[0]  # Assume square
    radius = int(size * radius_percent / 100)
    
    # Create a mask for rounded corners
    mask = Image.new('L', (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle([(0, 0), (size, size)], radius=radius, fill=255)
    
    # Apply the mask
    result = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    result.paste(image, (0, 0))
    result.putalpha(mask)
    
    return result

def add_shadow_effect(image, shadow_color=(0, 0, 0, 60), offset=(2, 2), blur_radius=4):
    """Add a subtle shadow effect to the icon"""
    size = image.size
    
    # Create shadow layer
    shadow = Image.new('RGBA', (size[0] + blur_radius * 2, size[1] + blur_radius * 2), (0, 0, 0, 0))
    shadow_draw = ImageDraw.Draw(shadow)
    
    # Draw shadow (simplified)
    shadow_layer = Image.new('RGBA', size, shadow_color)
    shadow_layer = shadow_layer.filter(ImageFilter.GaussianBlur(blur_radius))
    
    # Composite
    result = Image.new('RGBA', size, (0, 0, 0, 0))
    result.paste(shadow_layer, offset, shadow_layer)
    result.paste(image, (0, 0), image)
    
    return result

def generate_png_from_svg(svg_path, size, output_path, rounded=False, shadow=False):
    """Convert SVG to PNG at specified size"""
    print(f"Generating {output_path.name} ({size}x{size})")
    
    # Convert SVG to PNG using cairosvg
    png_data = cairosvg.svg2png(
        url=str(svg_path),
        output_width=size,
        output_height=size,
        background_color='transparent'
    )
    
    # Load with PIL for post-processing
    with tempfile.NamedTemporaryFile(suffix='.png') as temp_file:
        temp_file.write(png_data)
        temp_file.flush()
        
        image = Image.open(temp_file.name).convert('RGBA')
        
        # Post-processing effects
        if rounded:
            image = create_rounded_icon(image)
        
        if shadow:
            image = add_shadow_effect(image)
        
        # Save final result
        image.save(output_path, 'PNG', optimize=True)

def generate_icns(png_512_path, icns_path):
    """Generate macOS .icns file from 512px PNG"""
    try:
        # Try using iconutil (macOS only)
        import subprocess
        
        # Create iconset directory
        iconset_dir = icns_path.parent / f"{icns_path.stem}.iconset"
        iconset_dir.mkdir(exist_ok=True)
        
        # Define required sizes for iconset
        iconset_sizes = [
            (16, "icon_16x16.png"),
            (32, "icon_16x16@2x.png"),
            (32, "icon_32x32.png"),
            (64, "icon_32x32@2x.png"),
            (128, "icon_128x128.png"),
            (256, "icon_128x128@2x.png"),
            (256, "icon_256x256.png"),
            (512, "icon_256x256@2x.png"),
            (512, "icon_512x512.png"),
            (1024, "icon_512x512@2x.png"),
        ]
        
        # Generate each size
        base_image = Image.open(png_512_path)
        for size, filename in iconset_sizes:
            if size <= 512:
                resized = base_image.resize((size, size), Image.Resampling.LANCZOS)
            else:
                # For 1024px, upscale carefully
                resized = base_image.resize((size, size), Image.Resampling.LANCZOS)
            
            resized.save(iconset_dir / filename, 'PNG', optimize=True)
        
        # Convert to .icns using iconutil
        result = subprocess.run([
            'iconutil', '-c', 'icns', str(iconset_dir), '-o', str(icns_path)
        ], capture_output=True, text=True)
        
        if result.returncode == 0:
            print(f"Generated {icns_path.name} successfully")
            # Clean up iconset directory
            import shutil
            shutil.rmtree(iconset_dir)
        else:
            print(f"Warning: iconutil failed: {result.stderr}")
            
    except Exception as e:
        print(f"Warning: Could not generate .icns file: {e}")

def generate_ico(png_512_path, ico_path):
    """Generate Windows .ico file"""
    try:
        base_image = Image.open(png_512_path)
        
        # Create multiple sizes for .ico
        sizes = [16, 24, 32, 48, 64, 128, 256]
        images = []
        
        for size in sizes:
            resized = base_image.resize((size, size), Image.Resampling.LANCZOS)
            images.append(resized)
        
        # Save as .ico
        images[0].save(
            ico_path, 
            format='ICO', 
            sizes=[(img.size[0], img.size[1]) for img in images],
            bitmap_format='png'
        )
        print(f"Generated {ico_path.name} successfully")
        
    except Exception as e:
        print(f"Warning: Could not generate .ico file: {e}")

def main():
    """Generate all required app icons"""
    
    # Check for required dependencies
    try:
        import cairosvg
        from PIL import Image
    except ImportError:
        print("Error: Required dependencies not found.")
        print("Please install: pip install Pillow cairosvg")
        sys.exit(1)
    
    # Paths
    project_root = Path(__file__).parent.parent
    svg_path = project_root / "design" / "logo.svg"
    icons_dir = project_root / "src-tauri" / "icons"
    
    if not svg_path.exists():
        print(f"Error: SVG file not found at {svg_path}")
        sys.exit(1)
    
    icons_dir.mkdir(exist_ok=True)
    
    print("🔥 Code Furnace Icon Generator")
    print("=" * 40)
    
    # Generate core PNG sizes
    sizes = [
        (32, "32x32.png", False, False),
        (128, "128x128.png", True, True),
        (256, "128x128@2x.png", True, True),
        (512, "icon.png", True, True),
    ]
    
    for size, filename, rounded, shadow in sizes:
        output_path = icons_dir / filename
        generate_png_from_svg(svg_path, size, output_path, rounded, shadow)
    
    # Generate Windows Store logos (Microsoft requires specific formats)
    store_sizes = [
        (30, "Square30x30Logo.png"),
        (44, "Square44x44Logo.png"),
        (71, "Square71x71Logo.png"),
        (89, "Square89x89Logo.png"),
        (107, "Square107x107Logo.png"),
        (142, "Square142x142Logo.png"),
        (150, "Square150x150Logo.png"),
        (284, "Square284x284Logo.png"),
        (310, "Square310x310Logo.png"),
        (50, "StoreLogo.png"),
    ]
    
    for size, filename in store_sizes:
        output_path = icons_dir / filename
        generate_png_from_svg(svg_path, size, output_path, rounded=True)
    
    # Generate platform-specific formats
    print("\nGenerating platform-specific formats...")
    
    # macOS .icns
    generate_icns(icons_dir / "icon.png", icons_dir / "icon.icns")
    
    # Windows .ico
    generate_ico(icons_dir / "icon.png", icons_dir / "icon.ico")
    
    print("\n✨ Icon generation complete!")
    print(f"Generated icons in: {icons_dir}")
    print("\nNext steps:")
    print("1. Review the generated icons")
    print("2. Run: npm run tauri:build")
    print("3. Your app will use the new volcanic logo!")

if __name__ == "__main__":
    main()