#include "common.h"

#ifndef JUKEBOX_LCD_H
#define JUKEBOX_LCD_H

// Screen states, for what to display, when to display it, and how!
typedef enum
{
	Unknown,
	WaitingConnection,
	ShowStats,
} ScreenState;

void lcd_init(void);

void lcd_set_color(uint8_t r, uint8_t g, uint8_t b);

void lcd_on(void);
void lcd_off(void);
void lcd_clear(void);
void lcd_present(void);

void lcd_put(uint16_t x, uint16_t y);

void lcd_rect(uint16_t x, uint16_t y, uint16_t w, uint16_t h);

void lcd_print(char * text, uint16_t x, uint16_t y, uint8_t s);
void lcd_print_raw(char * text, uint16_t x, uint16_t y, uint8_t s);

void lcd_task(void);

#endif // JUKEBOX_LCD_H
