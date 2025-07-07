use crate::gb::ram::{RAM, INTERRUPT_FLAGS_ADDRESS};

const VRAM_SIZE: usize = 0x2000;
const VRAM_ADDRESS: u16 = 0x8000;
const OAM_SIZE: usize = 0xA0;
const OAM_ADDRESS: u16 = 0xFE00;

const LCDC_ADDRESS: u16 = 0xFF40; // LCD Control
const LY_ADDRESS: u16 = 0xFF44; // LCD Y Coordinate (read only)
const LYC_ADDRESS: u16 = 0xFF45; // LY Compare
const LCD_STATUS_ADDRESS: u16 = 0xFF41; // LCD Status

const CYCLES_OAM: u32 = 80;      // Mode 2 - OAM Search
const CYCLES_VRAM: u32 = 172;    // Mode 3 - Pixel Transfer (minimum)
const CYCLES_HBLANK: u32 = 204;  // Mode 0 - Horizontal Blank
const CYCLES_VBLANK: u32 = 4560; // Mode 1 - Vertical Blank
const SCANLINES_DISPLAY: u8 = 144;  // Visible scanlines
const MAX_SCANLINES: u8 = 154;      // Total scanlines per frame
const SCANLINE_SIZE: u8 = 160;      // Number of pixels in a scanline

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Mode {
  HBLANK = 0,
  VBLANK = 1,
  OAM = 2,
  VRAM = 3,
}

struct GBTile {
  pub lines: [u16; 8],
}

pub struct LCDC_REG {
  pub bg_enable: bool,
  pub obj_enable: bool,
  pub obj_size: bool,
  pub bg_tile_map_display_select: bool,
  pub bg_tile_data_select: bool,
  pub window_enable: bool,
  pub window_tile_map_display_select: bool,
}

enum LCDC_MASKS {
  BG_ENABLE = 0x01,
  OBJ_ENABLE = 0x02,
  OBJ_SIZE = 0x04,
  BG_TILE_MAP_DISPLAY_SELECT = 0x08,
  BG_TILE_DATA_SELECT = 0x10,
  WINDOW_ENABLE = 0x20,
  WINDOW_TILE_MAP_DISPLAY_SELECT = 0x40,
}

impl std::convert::From<u8> for LCDC_REG {
  fn from(value: u8) -> Self {
    LCDC_REG {
      bg_enable: (value & 0x01) != 0,
      obj_enable: (value & 0x02) != 0,
      obj_size: (value & 0x04) != 0,
      bg_tile_map_display_select: (value & 0x08) != 0,
      bg_tile_data_select: (value & 0x10) != 0,
      window_enable: (value & 0x20) != 0,
      window_tile_map_display_select: (value & 0x40) != 0,
    }
  }
}

impl std::convert::From<LCDC_REG> for u8 {
  fn from(lcdc: LCDC_REG) -> u8 {
    let mut value = 0;
    value |= (lcdc.bg_enable as u8) << (LCDC_MASKS::BG_ENABLE as u8).trailing_zeros();
    value |= (lcdc.obj_enable as u8) << (LCDC_MASKS::OBJ_ENABLE as u8).trailing_zeros();
    value |= (lcdc.obj_size as u8) << (LCDC_MASKS::OBJ_SIZE as u8).trailing_zeros();
    value |= (lcdc.bg_tile_map_display_select as u8) << (LCDC_MASKS::BG_TILE_MAP_DISPLAY_SELECT as u8).trailing_zeros();
    value |= (lcdc.bg_tile_data_select as u8) << (LCDC_MASKS::BG_TILE_DATA_SELECT as u8).trailing_zeros();
    value |= (lcdc.window_enable as u8) << (LCDC_MASKS::WINDOW_ENABLE as u8).trailing_zeros();
    value |= (lcdc.window_tile_map_display_select as u8) << (LCDC_MASKS::WINDOW_TILE_MAP_DISPLAY_SELECT as u8).trailing_zeros();
    
    value
  }
}


pub struct LCD_STATUS_REG {
  pub mode: Mode,
  pub ly_compare: bool,
  pub mode_0_set: bool,
  pub mode_1_set: bool,
  pub mode_2_set: bool,
  pub lyc_int_select: bool,
  pub empty_1: bool,
}

pub enum LCD_STATUS_MASKS {
  MODE = 0b00000011,
  LY_COMPARE = 0b00000100,
  MODE_0_SET = 0b00001000,
  MODE_1_SET = 0b00010000,
  MODE_2_SET = 0b00100000,
  LYC_INT_SELECT = 0b01000000,
}

impl std::convert::From<LCD_STATUS_REG> for u8 {
  fn from(status: LCD_STATUS_REG) -> u8 {
    let mut value = 0;
    value |= (status.mode as u8) << ((LCD_STATUS_MASKS::MODE as u8).trailing_zeros());
    value |= (if status.ly_compare { 1 } else { 0 }) << ((LCD_STATUS_MASKS::LY_COMPARE as u8).trailing_zeros());
    value |= (if status.mode_0_set { 1 } else { 0 }) << ((LCD_STATUS_MASKS::MODE_0_SET as u8).trailing_zeros());
    value |= (if status.mode_1_set { 1 } else { 0 }) << ((LCD_STATUS_MASKS::MODE_1_SET as u8).trailing_zeros());
    value |= (if status.mode_2_set { 1 } else { 0 }) << ((LCD_STATUS_MASKS::MODE_2_SET as u8).trailing_zeros());
    value |= (if status.lyc_int_select { 1 } else { 0 }) << ((LCD_STATUS_MASKS::LYC_INT_SELECT as u8).trailing_zeros());
    value
  }
}

impl std::convert::From<u8> for LCD_STATUS_REG {
    fn from(value: u8) -> Self {
        LCD_STATUS_REG {
            mode: match value & (LCD_STATUS_MASKS::MODE as u8) {
                0 => Mode::HBLANK,
                1 => Mode::VBLANK,
                2 => Mode::OAM,
                3 => Mode::VRAM,
                _ => Mode::HBLANK, // fallback
            },
            ly_compare: (value & LCD_STATUS_MASKS::LY_COMPARE as u8) != 0,
            mode_0_set: (value & LCD_STATUS_MASKS::MODE_0_SET as u8) != 0,
            mode_1_set: (value & LCD_STATUS_MASKS::MODE_1_SET as u8) != 0,
            mode_2_set: (value & LCD_STATUS_MASKS::MODE_2_SET as u8) != 0,
            lyc_int_select: (value & LCD_STATUS_MASKS::LYC_INT_SELECT as u8) != 0,
            empty_1: false,
        }
    }
}

pub struct GPU<'a> {
    pub ram: &'a mut RAM,
    pub vram: [u8; VRAM_SIZE],
    pub oam: [u8; OAM_SIZE],
    pub clock: u32,
    pub mode: Mode,
    current_scanline: u8,
    pub screen_buffer: Vec<u8>,  // Buffer for the current frame
}

impl<'a> GPU<'a> {
    pub fn new(ram: &'a mut RAM) -> Self {
        // Initialise LCD Status Register in memory
        let lcd_status = LCD_STATUS_REG {
            mode: Mode::OAM,
            ly_compare: false,
            mode_0_set: false,
            mode_1_set: false,
            mode_2_set: false,
            lyc_int_select: false,
            empty_1: false,
        };
        ram.write(LCD_STATUS_ADDRESS, lcd_status.into());

        Self {
            ram: ram,
            vram: [0; VRAM_SIZE],
            oam: [0; OAM_SIZE],
            clock: 0,
            mode: Mode::OAM,
            current_scanline: 0,
            screen_buffer: vec![0; SCANLINE_SIZE as usize * SCANLINES_DISPLAY as usize * 4], // 160x144 pixels, 4 bytes per pixel (RGBA)
        }
    }

    #[cfg(test)]
    pub fn set_current_scanline(&mut self, value: u8) {
        self.current_scanline = value;
    }

    #[cfg(test)]
    pub fn get_current_scanline(&self) -> u8 {
        self.current_scanline
    }

    pub fn step(&mut self, cycles: u32) {
        self.clock += cycles;
        self.step_set_mode();
        self.step_lcd_status();
    }

    fn step_set_mode(&mut self) {
        match self.mode {
            Mode::OAM => {
                if self.clock >= CYCLES_OAM {
                    self.mode = Mode::VRAM;
                    self.clock = 0;
                }
            }
            Mode::VRAM => {
                if self.clock >= CYCLES_VRAM {
                    self.mode = Mode::HBLANK;
                    self.clock = 0;
                }
            }
            Mode::HBLANK => {
                if self.clock >= CYCLES_HBLANK {
                    self.render_scanline();
                    self.current_scanline += 1;
                    self.clock = 0;
                    if self.current_scanline >= SCANLINES_DISPLAY {
                        self.mode = Mode::VBLANK;
                        // Trigger V-Blank interrupt
                        let mut if_flags = self.ram.read(INTERRUPT_FLAGS_ADDRESS);
                        if_flags |= 0x01; // Set VBlank interrupt flag
                        self.ram.write(INTERRUPT_FLAGS_ADDRESS, if_flags);
                    } else {
                        self.mode = Mode::OAM;
                    }
                }
            }
            Mode::VBLANK => {
                if self.clock >= CYCLES_VBLANK {
                    self.current_scanline += 1;
                    self.clock = 0;
                    if self.current_scanline >= MAX_SCANLINES {
                        // Frame complete, start new frame
                        self.mode = Mode::OAM;
                        self.current_scanline = 0;
                        // TODO: Update screen with screen_buffer
                    }
                }
            }
        }
    }

    fn step_lcd_status(&mut self) {
        let mut lcd_status = self.get_lcd_status();
        lcd_status.mode = self.mode;
        self.ram.write(LY_ADDRESS, self.current_scanline);
        lcd_status.ly_compare = self.current_scanline == self.ram.read(LYC_ADDRESS);
        self.ram.write(LCD_STATUS_ADDRESS, lcd_status.into());
    }

    pub fn render_scanline(&mut self) {
        let lcdc = self.get_lcdc();
        
        // Render background if enabled
        if lcdc.bg_enable {
            self.render_background();
        }

        // Render sprites if enabled
        if lcdc.obj_enable {
            self.render_sprites();
        }
    }

    fn render_background(&mut self) {
        let lcdc = self.get_lcdc();
        let y = self.current_scanline;
        
        // Get the base address for the tile map
        let tile_map_addr = if lcdc.bg_tile_map_display_select {
            0x9C00
        } else {
            0x9800
        };

        // Get the base address for tile data
        let tile_data_addr = if lcdc.bg_tile_data_select {
            0x8000
        } else {
            0x8800
        };

        // For each pixel in the scanline
        for x in 0..SCANLINE_SIZE {
            // Calculate tile coordinates
            let tile_x = (x / 8) as u8;
            let tile_y = (y / 8) as u8;
            
            // Get tile number from tile map
            let tile_map_index = (tile_y as u16 * 32 + tile_x as u16) + tile_map_addr;
            let tile_number = self.read_vram(tile_map_index as u16);
            
            // Get tile data
            let tile_addr = if lcdc.bg_tile_data_select {
                tile_data_addr + (tile_number as u16 * 16)
            } else {
                tile_data_addr + ((tile_number as i8 as i16 + 128) * 16) as u16
            };

            // Get pixel position within tile
            let pixel_x = x % 8;
            let pixel_y = y % 8;

            // Get pixel data from tile
            let tile_line = self.read_vram(tile_addr + (pixel_y * 2) as u16);
            let tile_line_high = self.read_vram(tile_addr + (pixel_y * 2 + 1) as u16);
            
            // Get color number for this pixel
            let color_bit = 7 - pixel_x;
            let color_number = ((tile_line_high >> color_bit) & 1) << 1 | ((tile_line >> color_bit) & 1);

            // Convert color number to RGBA (using a simple grayscale palette for now)
            let color = match color_number {
                0 => [0xFF, 0xFF, 0xFF, 0xFF], // White
                1 => [0xCC, 0xCC, 0xCC, 0xFF], // Light gray
                2 => [0x77, 0x77, 0x77, 0xFF], // Dark gray
                3 => [0x00, 0x00, 0x00, 0xFF], // Black
                _ => [0x00, 0x00, 0x00, 0xFF],
            };

            // Write to screen buffer
            let screen_index = (y as usize * SCANLINE_SIZE as usize + (x as usize )* 4 as usize);
            self.screen_buffer[screen_index..screen_index + 4].copy_from_slice(&color);
        }
    }

    fn render_sprites(&mut self) {
        let lcdc = self.get_lcdc();
        let y = self.current_scanline;
        let sprite_height = if lcdc.obj_size { 16 } else { 8 };
        let sprite_width = 8;

        // Find sprites visible on this scanline
        let mut visible_sprites = Vec::new();
        for sprite_index in 0..40 {
            let sprite_addr = sprite_index * 4 + OAM_ADDRESS;
            
            // Get sprite Y position
            let sprite_y = self.read_oam(sprite_addr) as i16 - 16;
            if sprite_y <= y as i16 && sprite_y + sprite_height as i16 > y as i16 {
                visible_sprites.push(sprite_index);
            }
        }

        // Sort sprites by X coordinate (for proper priority)
        visible_sprites.sort_by_key(|&i| self.read_oam(i * 4 + 1 + OAM_ADDRESS));

        // Process up to 10 sprites per scanline
        for &sprite_index in visible_sprites.iter().take(10) {
            let sprite_addr = sprite_index * 4 + OAM_ADDRESS;
            
            let sprite_y = self.read_oam(sprite_addr) as i16 - 16;
            let sprite_x = self.read_oam(sprite_addr + 1) as i16 - 8;
            let tile_number = self.read_oam(sprite_addr + 2);
            let attributes = self.read_oam(sprite_addr + 3);

            let priority = (attributes & 0x80) == 0; // 0 = above background, 1 = below background
            let y_flip = (attributes & 0x40) != 0;
            let x_flip = (attributes & 0x20) != 0;
            let palette = (attributes & 0x10) != 0;

            // Calculate tile data address
            let tile_addr = 0x8000 + (tile_number as u16 * 16);

            // Get pixel position within sprite
            let mut pixel_y = (y as i16 - sprite_y) as u8;
            if y_flip {
                pixel_y = sprite_height - 1 - pixel_y;
            }

            // For each pixel in the sprite's width
            for x in 0..8 {
                if ((sprite_x + x as i16) < 0) || ((sprite_x + x as i16) >= SCANLINE_SIZE as i16) {
                    continue;
                }

                let mut pixel_x = x;
                if x_flip {
                    pixel_x = sprite_width - 1 - x;
                }
                
                // Get pixel data from tile
                let tile_line = self.read_vram(tile_addr + (pixel_y * 2) as u16);
                let tile_line_high = self.read_vram(tile_addr + (pixel_y * 2 + 1) as u16);

                // Get color number for this pixel
                let color_bit = 7 - pixel_x;
                let color_number = ((tile_line_high >> color_bit) & 1) << 1 | ((tile_line >> color_bit) & 1);

                // Skip transparent pixels (color 0)
                if color_number == 0 {
                    continue;
                }

                // Convert color number to RGBA (using a simple grayscale palette for now)
                let color = match color_number {
                    1 => [0xCC, 0xCC, 0xCC, 0xFF], // Light gray
                    2 => [0x77, 0x77, 0x77, 0xFF], // Dark gray
                    3 => [0x00, 0x00, 0x00, 0xFF], // Black
                    _ => continue,
                };

                // Write to screen buffer if priority allows
                let screen_x = sprite_x + x as i16;
                if screen_x >= 0 && screen_x < SCANLINE_SIZE as i16 {
                    let screen_index = (y as usize * SCANLINE_SIZE as usize + screen_x as usize) * 4;
                    if priority || self.screen_buffer[screen_index] == 0xFF {
                        self.screen_buffer[screen_index..screen_index + 4].copy_from_slice(&color);
                    }
                }
            }
        }
    }

    pub fn read_vram(&self, address: u16) -> u8 {
        // Only allow VRAM access during H-Blank and V-Blank
        assert!(VRAM_ADDRESS <= address && address < VRAM_ADDRESS + VRAM_SIZE as u16);
        if self.mode == Mode::HBLANK || self.mode == Mode::VBLANK {
            self.vram[(address - VRAM_ADDRESS) as usize]
        } else {
            0xFF // Return 0xFF if accessed during restricted modes
        }
    }

    pub fn write_vram(&mut self, address: u16, value: u8) {
        // Only allow VRAM access during H-Blank and V-Blank
        assert!(VRAM_ADDRESS <= address && address < VRAM_ADDRESS + VRAM_SIZE as u16);
        if self.mode == Mode::HBLANK || self.mode == Mode::VBLANK {
            self.vram[(address - VRAM_ADDRESS) as usize] = value;
        }
    }

    pub fn read_oam(&self, address: u16) -> u8 {
        // Only allow OAM access during H-Blank and V-Blank
        if self.mode == Mode::HBLANK || self.mode == Mode::VBLANK {
            self.oam[(address - OAM_ADDRESS) as usize]
        } else {
            0xFF // Return 0xFF if accessed during restricted modes
        }
    }

    pub fn write_oam(&mut self, address: u16, value: u8) {
        // Only allow OAM access during H-Blank and V-Blank
        if self.mode == Mode::HBLANK || self.mode == Mode::VBLANK {
            self.oam[(address - OAM_ADDRESS) as usize] = value;
        }
    }

    pub fn get_mode(&self) -> u8 {
        self.mode as u8
    }

    pub fn get_lcdc(&self) -> LCDC_REG {
        let lcdc = self.ram.read(LCDC_ADDRESS);
        LCDC_REG::from(lcdc)    
    }

    pub fn get_lcd_status(&self) -> LCD_STATUS_REG {
        let lcd_status = self.ram.read(LCD_STATUS_ADDRESS);
        LCD_STATUS_REG::from(lcd_status)
    }

    pub fn set_lcdc(&mut self, value: u8) {
        self.ram.write(LCDC_ADDRESS, value);
    }

    pub fn set_lcd_status(&mut self, value: u8) {
        self.ram.write(LCD_STATUS_ADDRESS, value);
    }

    // Add method to trigger LCD STAT interrupts
    fn trigger_lcd_stat_interrupt(&mut self) {
        let mut if_flags = self.ram.read(INTERRUPT_FLAGS_ADDRESS);
        if_flags |= 0x02; // Set LCD STAT interrupt flag
        self.ram.write(INTERRUPT_FLAGS_ADDRESS, if_flags);
    }

    /*
    // TODO: Not sure if this is needed 

    // Add method to check and trigger LCD STAT interrupts based on conditions
    fn check_lcd_stat_interrupts(&mut self, ram: &mut RAM) {
        let stat = self.get_stat();
        let ly = self.current_scanline;
        let lyc = self.get_lyc();

        let mut should_trigger = false;

        // Check various LCD STAT interrupt conditions
        if stat.lyc_ly_int && ly == lyc {
            should_trigger = true;
        }
        if stat.oam_int && self.mode == Mode::OAM {
            should_trigger = true;
        }
        if stat.vblank_int && self.mode == Mode::VBLANK {
            should_trigger = true;
        }
        if stat.hblank_int && self.mode == Mode::HBLANK {
            should_trigger = true;
        }

        if should_trigger {
            self.trigger_lcd_stat_interrupt(ram);
        }
    }
     */
}   