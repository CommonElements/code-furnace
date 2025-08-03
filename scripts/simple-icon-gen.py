#!/usr/bin/env python3
"""
Simple icon generator that creates PNG files from an SVG-inspired design
using only Pillow (no external SVG dependencies)
"""

from PIL import Image, ImageDraw, ImageFilter
from pathlib import Path
import math

def create_volcanic_icon(size):
    """Create a volcanic-themed icon using PIL drawing commands"""
    
    # Create base image with transparent background
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Scale factor for coordinates
    s = size / 512.0
    
    # Colors
    obsidian_dark = (13, 13, 13, 255)
    obsidian_medium = (42, 42, 42, 255)
    lava_primary = (255, 69, 0, 255)
    lava_secondary = (255, 140, 0, 255)
    lava_gold = (255, 215, 0, 255)
    ember = (255, 112, 67, 255)
    
    # Background circle (obsidian)
    center = size // 2
    bg_radius = int(240 * s)
    draw.ellipse([center - bg_radius, center - bg_radius, 
                  center + bg_radius, center + bg_radius], 
                 fill=obsidian_dark)
    
    # Outer ring
    ring_radius = int(220 * s)
    ring_width = max(1, int(3 * s))
    for i in range(ring_width):
        draw.ellipse([center - ring_radius + i, center - ring_radius + i,
                      center + ring_radius - i, center + ring_radius - i],
                     outline=lava_primary)
    
    # Forge anvil base
    anvil_left = int(160 * s)
    anvil_right = int(352 * s)
    anvil_top = int(332 * s)
    anvil_bottom = int(380 * s)
    draw.rectangle([anvil_left, anvil_top, anvil_right, anvil_bottom],
                   fill=obsidian_dark, outline=lava_primary, width=max(1, int(2 * s)))
    
    # Main furnace body (simplified polygon)
    furnace_points = [
        (int(200 * s), int(320 * s)),  # bottom left
        (int(312 * s), int(320 * s)),  # bottom right
        (int(320 * s), int(180 * s)),  # right side
        (int(290 * s), int(120 * s)),  # top right slope
        (int(256 * s), int(100 * s)),  # top center
        (int(222 * s), int(120 * s)),  # top left slope
        (int(192 * s), int(180 * s)),  # left side
    ]
    draw.polygon(furnace_points, fill=obsidian_medium, outline=lava_secondary, width=max(1, int(3 * s)))
    
    # Lava core (glowing center) - create gradient effect with multiple ellipses
    core_center_x, core_center_y = int(256 * s), int(200 * s)
    core_width, core_height = int(45 * s), int(60 * s)
    
    # Create gradient effect with multiple ellipses
    for i in range(5):
        alpha = 255 - i * 40
        radius_w = core_width - i * int(5 * s)
        radius_h = core_height - i * int(8 * s)
        if radius_w > 0 and radius_h > 0 and alpha > 0:
            color = lava_gold if i < 2 else lava_secondary if i < 4 else lava_primary
            color = (*color[:3], alpha)
            draw.ellipse([core_center_x - radius_w, core_center_y - radius_h,
                          core_center_x + radius_w, core_center_y + radius_h],
                         fill=color)
    
    # Molten cracks (simplified wavy lines)
    crack_y_positions = [int(160 * s), int(200 * s), int(240 * s)]
    crack_colors = [lava_gold, lava_secondary, lava_primary]
    crack_widths = [max(1, int(3 * s)), max(1, int(4 * s)), max(1, int(3 * s))]
    
    for i, (y_pos, color, width) in enumerate(zip(crack_y_positions, crack_colors, crack_widths)):
        # Create wavy crack line
        points = []
        for x in range(int(210 * s), int(302 * s), max(1, int(10 * s))):
            offset = int(10 * s * math.sin((x - 210 * s) * 0.1))
            points.extend([x, y_pos + offset])
        
        if len(points) >= 4:  # Need at least 2 points
            for j in range(0, len(points) - 2, 2):
                draw.line([points[j], points[j+1], points[j+2], points[j+3]], 
                         fill=color, width=width)
    
    # Forge opening (furnace mouth)
    mouth_center_x, mouth_center_y = int(256 * s), int(140 * s)
    mouth_width, mouth_height = int(25 * s), int(15 * s)
    draw.ellipse([mouth_center_x - mouth_width, mouth_center_y - mouth_height,
                  mouth_center_x + mouth_width, mouth_center_y + mouth_height],
                 fill=(0, 0, 0, 255), outline=lava_primary, width=max(1, int(2 * s)))
    
    # Inner glow in mouth
    inner_width, inner_height = int(20 * s), int(10 * s)
    draw.ellipse([mouth_center_x - inner_width, mouth_center_y - inner_height,
                  mouth_center_x + inner_width, mouth_center_y + inner_height],
                 fill=(*lava_secondary[:3], 150))
    
    # Sparks/embers (small circles)
    ember_positions = [
        (int(200 * s), int(120 * s), int(3 * s)),
        (int(320 * s), int(100 * s), int(2 * s)),
        (int(280 * s), int(80 * s), int(2.5 * s)),
        (int(180 * s), int(90 * s), int(2 * s)),
        (int(340 * s), int(130 * s), int(1.5 * s)),
    ]
    
    ember_colors = [lava_gold, lava_secondary, lava_gold, lava_primary, lava_secondary]
    
    for (x, y, radius), color in zip(ember_positions, ember_colors):
        if radius > 0:
            draw.ellipse([x - radius, y - radius, x + radius, y + radius], fill=color)
    
    # Furnace vents (cooling slits)
    vent_y = int(300 * s)
    vent_height = max(1, int(3 * s))
    vent_width = int(20 * s)
    vent_positions = [int(200 * s), int(230 * s), int(260 * s), int(290 * s)]
    
    for x in vent_positions:
        draw.rectangle([x, vent_y, x + vent_width, vent_y + vent_height],
                       fill=(*lava_primary[:3], 150))
    
    # Central focal point (bright core)
    core_radius = max(1, int(12 * s))
    bright_radius = max(1, int(6 * s))
    
    draw.ellipse([center - core_radius, core_center_y - core_radius,
                  center + core_radius, core_center_y + core_radius],
                 fill=lava_gold)
    draw.ellipse([center - bright_radius, core_center_y - bright_radius,
                  center + bright_radius, core_center_y + bright_radius],
                 fill=(255, 255, 255, 200))
    
    return img

def create_rounded_icon(image, radius_percent=20):
    """Create a rounded corner version of the image"""
    size = image.size[0]
    radius = int(size * radius_percent / 100)
    
    # Create mask
    mask = Image.new('L', (size, size), 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.rounded_rectangle([(0, 0), (size, size)], radius=radius, fill=255)
    
    # Apply mask
    result = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    result.paste(image, (0, 0))
    result.putalpha(mask)
    
    return result

def main():
    """Generate all required icons"""
    print("🔥 Code Furnace Simple Icon Generator")
    print("=" * 45)
    
    # Paths
    project_root = Path(__file__).parent.parent
    icons_dir = project_root / "src-tauri" / "icons"
    icons_dir.mkdir(exist_ok=True)
    
    # Icon configurations: (size, filename, rounded, description)
    icon_configs = [
        (32, "32x32.png", False, "Small app icon"),
        (128, "128x128.png", True, "Medium app icon"),
        (256, "128x128@2x.png", True, "Retina medium icon"),
        (512, "icon.png", True, "Large app icon"),
        (30, "Square30x30Logo.png", True, "Windows Store 30x30"),
        (44, "Square44x44Logo.png", True, "Windows Store 44x44"),
        (71, "Square71x71Logo.png", True, "Windows Store 71x71"),
        (89, "Square89x89Logo.png", True, "Windows Store 89x89"),
        (107, "Square107x107Logo.png", True, "Windows Store 107x107"),
        (142, "Square142x142Logo.png", True, "Windows Store 142x142"),
        (150, "Square150x150Logo.png", True, "Windows Store 150x150"),
        (284, "Square284x284Logo.png", True, "Windows Store 284x284"),
        (310, "Square310x310Logo.png", True, "Windows Store 310x310"),
        (50, "StoreLogo.png", True, "Windows Store Logo")
    ]
    
    for size, filename, rounded, description in icon_configs:
        print(f"Generating {filename} ({size}x{size}) - {description}")
        
        # Create the volcanic icon
        icon = create_volcanic_icon(size)
        
        # Apply rounding if needed
        if rounded:
            icon = create_rounded_icon(icon)
        
        # Save
        output_path = icons_dir / filename
        icon.save(output_path, 'PNG', optimize=True)
    
    # Create .ico file (Windows)
    print("Generating icon.ico for Windows...")
    try:
        # Create multiple sizes for .ico
        ico_sizes = [16, 24, 32, 48, 64, 128, 256]
        ico_images = []
        
        for size in ico_sizes:
            img = create_volcanic_icon(size)
            ico_images.append(img)
        
        ico_path = icons_dir / "icon.ico"
        ico_images[0].save(
            ico_path,
            format='ICO',
            sizes=[(img.size[0], img.size[1]) for img in ico_images]
        )
        print(f"Generated {ico_path.name}")
        
    except Exception as e:
        print(f"Warning: Could not generate .ico file: {e}")
    
    print("\n✨ Icon generation complete!")
    print(f"Generated {len(icon_configs) + 1} icon files in: {icons_dir}")
    print("\nNext steps:")
    print("1. Review the generated volcanic icons")
    print("2. Run: npm run tauri:build")
    print("3. Your Code Furnace app now has a molten logo! 🌋")

if __name__ == "__main__":
    main()