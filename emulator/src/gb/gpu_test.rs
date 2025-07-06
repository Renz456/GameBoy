#[cfg(test)]
mod tests {
    use crate::gb::gpu::{GPU, Mode, LCDC_REG, LCD_STATUS_REG};
    use crate::gb::ram::RAM;

    // Helper function to create a GPU with specific initial state
    fn create_gpu_with_state(
        mode: Mode,
        current_scanline: u8,
        clock: u32,
        ram: &mut RAM
    ) -> GPU {
        let mut gpu = GPU::new(ram);
        gpu.mode = mode;
        gpu.set_current_scanline(current_scanline);
        gpu.clock = clock;
        gpu
    }

    // Helper function to write a tile to VRAM
    fn write_tile(gpu: &mut GPU, tile_number: u8, tile_data: &[u8; 16]) {
        let base_addr = 0x8000 + (tile_number as u16 * 16);
        for (i, &byte) in tile_data.iter().enumerate() {
            gpu.write_vram(base_addr + i as u16, byte);
        }
    }

    // Helper function to write a sprite to OAM
    fn write_sprite(gpu: &mut GPU, sprite_index: usize, y: u8, x: u8, tile_number: u8, attributes: u8) {
        let base_addr = 0xFE00 + (sprite_index * 4) as u16;
        gpu.write_oam(base_addr, y);
        gpu.write_oam(base_addr + 1, x);
        gpu.write_oam(base_addr + 2, tile_number);
        gpu.write_oam(base_addr + 3, attributes);
    }

    #[test]
    fn test_mode_transitions() {
        let mut ram = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::OAM, 0, 0, &mut ram);
        
        // Test OAM -> VRAM transition
        gpu.step(80);
        assert_eq!(gpu.mode, Mode::VRAM, "Should transition to VRAM mode");
        assert_eq!(gpu.clock, 0, "Clock should reset");

        // Test VRAM -> HBLANK transition
        gpu.step(172);
        assert_eq!(gpu.mode, Mode::HBLANK, "Should transition to HBLANK mode");
        assert_eq!(gpu.clock, 0, "Clock should reset");
        
        // Test HBLANK -> OAM transition (for next scanline)
        gpu.step(204);
        assert_eq!(gpu.get_current_scanline(), 1, "Scanline should increment");
        assert_eq!(gpu.mode, Mode::OAM, "Should transition back to OAM mode");
        assert_eq!(gpu.clock, 0, "Clock should reset");
    }

    #[test]
    fn test_vblank_transition() {
        let mut ram = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::HBLANK, 153, 0, &mut ram);
        
        // Trigger VBLANK
        gpu.step(204);
        assert_eq!(gpu.mode, Mode::VBLANK, "Should transition to VBLANK mode");
        assert_eq!(gpu.get_current_scanline(), 154, "Should be at first VBLANK scanline");

        // Test VBLANK duration
        gpu.step(4560);
        assert_eq!(gpu.mode, Mode::OAM, "Should transition back to OAM mode");
        assert_eq!(gpu.get_current_scanline(), 0, "Should reset scanline counter");
    }

    #[test]
    fn test_tile_rendering() {
        let mut ram = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::HBLANK, 0, 0, &mut ram);
        
        // Create a simple tile pattern (checkerboard)
        let tile_data = [
            0x55, 0x55, // 01010101, 01010101
            0x55, 0x55, // 01010101, 01010101
            0x55, 0xAA, // 01010101, 10101010
            0x55, 0xAA, // 01010101, 10101010
            0x55, 0xAA, // 01010101, 10101010
            0x55, 0xAA, // 01010101, 10101010
            0x55, 0xAA, // 01010101, 10101010
            0x55, 0xAA, // 01010101, 10101010
        ];

        // Write tile to VRAM
        write_tile(&mut gpu, 0, &tile_data);

        // Set up tile map
        for i in 0..20 {
            gpu.write_vram(0x9C00 + i as u16, 0); // Place tile 0 at (0,0)
        }

        // Enable background rendering
        let lcdc = LCDC_REG {
            bg_enable: true,
            obj_enable: false,
            obj_size: false,
            bg_tile_map_display_select: false,
            bg_tile_data_select: true,
            window_enable: false,
            window_tile_map_display_select: false,
        };
        gpu.set_lcdc(lcdc.into());

        // Render scanline
        gpu.render_scanline();

        // Check rendered pixels
        for x in 0..8 {
            let pixel_index = x * 4;
            let expected_color = if x % 2 == 0 {
                [0xFF, 0xFF, 0xFF, 0xFF] // White
            } else {
                [0x00, 0x00, 0x00, 0xFF] // Black
            };
            assert_eq!(
                &gpu.screen_buffer[pixel_index..pixel_index + 4],
                &expected_color,
                "Pixel index {:?} at x={} has wrong color",
                pixel_index, x
            );
        }
    }

    #[test]
    fn test_sprite_rendering() {
        let mut ram = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::HBLANK, 0, 0, &mut ram);
        
        // Create a simple sprite pattern
        let sprite_data = [
            0x3C, 0x3C, // 00111100, 00111100
            0xFF, 0xFF, // 11111111, 11111111
            0xFF, 0xFF, // 11111111, 11111111
            0x76, 0x34, // 01110110, 00110100
            0x00, 0x00, // 00000000, 00000000
            0x00, 0x00, // 00000000, 00000000
            0x00, 0x00, // 00000000, 00000000
            0x00, 0x00, // 00000000, 00000000
        ];

        // Write sprite to VRAM
        write_tile(&mut gpu, 0, &sprite_data);

        // Set up sprite in OAM
        write_sprite(&mut gpu, 0, 16, 8, 0, 0); // Y=16, X=8, Tile=0, Attributes=0

        // Enable sprite rendering
        let lcdc = LCDC_REG {
            bg_enable: false,
            obj_enable: true,
            obj_size: false,
            bg_tile_map_display_select: false,
            bg_tile_data_select: true,
            window_enable: false,
            window_tile_map_display_select: false,
        };
        gpu.set_lcdc(lcdc.into());

        // Render scanline
        gpu.render_scanline();

        // Check rendered sprite pixels
        for x in 0..8 {
            let pixel_index = ((x) * 4) as usize;
            let expected_color = if x < 2 || x > 5 {
                [0x00, 0x00, 0x00, 0x00] // Transparent
            } else {
                [0x00, 0x00, 0x00, 0xFF] // Black
            };
            println!("check x {:?}", x);
            println!("check screen buffer {:?} {:?} {:?} {:?}", gpu.screen_buffer[pixel_index], gpu.screen_buffer[pixel_index + 1], gpu.screen_buffer[pixel_index + 2], gpu.screen_buffer[pixel_index + 3]);
            assert_eq!(
                &gpu.screen_buffer[pixel_index..pixel_index + 4],
                &expected_color,
                "Sprite pixel at x={} has wrong color, pixel index {}",
                x, pixel_index
            );
        }
    }

    #[test]
    fn test_sprite_priority() {
        let mut ram: RAM = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::HBLANK, 0, 0, &mut ram);
        
        // Create two tiles: one for background, one for sprite
        let bg_tile = [0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF]; // Dark grey
        let sprite_tile = [0xFF; 16]; // Solid black

        // Write tiles to VRAM
        write_tile(&mut gpu, 0, &bg_tile);
        write_tile(&mut gpu, 1, &sprite_tile);
        // Set up background
        gpu.write_vram(0x9800, 0);

        // Set up two sprites at the same position
        write_sprite(&mut gpu, 0, 16, 8, 1, 0x80); // Priority: below background
        write_sprite(&mut gpu, 1, 16, 8, 1, 0x00); // Priority: above background
        
        // Enable both background and sprite rendering
        let lcdc = LCDC_REG {
            bg_enable: true,
            obj_enable: true,
            obj_size: false,
            bg_tile_map_display_select: false,
            bg_tile_data_select: true,
            window_enable: false,
            window_tile_map_display_select: false,
        };
        gpu.set_lcdc(lcdc.into());

        // Render scanline
        gpu.render_scanline();

        // Check that sprite with higher priority (0x00) is visible
        let pixel_index = (7 * 4) as usize;
        assert_eq!(
            &gpu.screen_buffer[pixel_index..pixel_index + 4],
            &[0x00, 0x00, 0x00, 0xFF], // Black (sprite color)
            "Higher priority sprite should be visible"
        );
    }

    #[test]
    fn test_sprite_flipping() {
        let mut ram = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::HBLANK, 0, 0, &mut ram);
        
        // Create a simple sprite pattern
        let sprite_data = [
            0xF0, 0x0F, // 11110000, 00001111 flipped -> 00001111, 11110000
            0x24, 0x18, // 00100100, 00011000
            0x18, 0x24, // 00011000, 00100100
            0x42, 0x81, // 01000010, 10000001
            0x00, 0x00, // 00000000, 00000000
            0x00, 0x00, // 00000000, 00000000
            0x00, 0x00, // 00000000, 00000000
            0x00, 0x00, // 00000000, 00000000
        ];

        // Write sprite to VRAM
        write_tile(&mut gpu, 0, &sprite_data);

        // Set up sprite with X flip
        write_sprite(&mut gpu, 0, 16, 8, 0, 0x20); // X flip enabled

        // Enable sprite rendering
        let lcdc = LCDC_REG {
            bg_enable: false,
            obj_enable: true,
            obj_size: false,
            bg_tile_map_display_select: false,
            bg_tile_data_select: true,
            window_enable: false,
            window_tile_map_display_select: false,
        };
        gpu.set_lcdc(lcdc.into());

        // Render scanline
        gpu.render_scanline();

        // Check that sprite is flipped horizontally
        for x in 0..8 {
            let pixel_index = ((x) * 4) as usize;
            let expected_color = if x < 4 {
                [0x77, 0x77, 0x77, 0xFF] // Dark gray
            } else {
                [0xCC, 0xCC, 0xCC, 0xFF] // Light gray
            };
            assert_eq!(
                &gpu.screen_buffer[pixel_index..pixel_index + 4],
                &expected_color,
                "Flipped sprite pixel at x={} has wrong color",
                x
            );
        }
    }

    #[test]
    fn test_vram_access_restrictions() {
        let mut ram = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::OAM, 0, 0, &mut ram);
        
        // Try to write to VRAM during OAM mode
        gpu.write_vram(0x8000, 0x42);
        assert_eq!(gpu.read_vram(0x8000), 0xFF, "VRAM should be inaccessible during OAM mode");

        // Switch to HBLANK mode
        gpu.mode = Mode::HBLANK;
        
        // Try to write to VRAM during HBLANK
        gpu.write_vram(0x8000, 0x42);
        assert_eq!(gpu.read_vram(0x8000), 0x42, "VRAM should be accessible during HBLANK");
    }

    #[test]
    fn test_oam_access_restrictions() {
        let mut ram = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::VRAM, 0, 0, &mut ram);
        
        // Try to write to OAM during VRAM mode
        gpu.write_oam(0xFE00, 0x42);
        assert_eq!(gpu.read_oam(0xFE00), 0xFF, "OAM should be inaccessible during VRAM mode");

        // Switch to HBLANK mode
        gpu.mode = Mode::HBLANK;
        
        // Try to write to OAM during HBLANK
        gpu.write_oam(0xFE00, 0x42);
        assert_eq!(gpu.read_oam(0xFE00), 0x42, "OAM should be accessible during HBLANK");
    }

    #[test]
    fn test_lcd_status_register() {
        let mut ram = RAM::new();
        let mut gpu = create_gpu_with_state(Mode::OAM, 0, 0, &mut ram);
        
        // Test mode bits
        let status = gpu.get_lcd_status();
        assert_eq!(status.mode as u8, Mode::OAM as u8, "LCD status mode should match current mode");

        // Test LY compare
        gpu.ram.write(0xFF45, 0x42); // Set LYC to 0x42
        gpu.set_current_scanline(0x42);
        gpu.step(1);
        let status = gpu.get_lcd_status();
        assert!(status.ly_compare, "LY compare flag should be set when LY equals LYC");

        // Test mode interrupts
        let status = LCD_STATUS_REG {
            mode: Mode::OAM,
            ly_compare: false,
            mode_0_set: true,
            mode_1_set: false,
            mode_2_set: false,
            lyc_int_select: false,
            empty_1: false,
        };
        gpu.set_lcd_status(status.into());
        let new_status = gpu.get_lcd_status();
        assert!(new_status.mode_0_set, "Mode 0 interrupt should be enabled");
    }
} 