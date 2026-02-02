# FinalFeedback

A self-hosted FFXIV feedback survey application built with Rust and Actix-web. Collect anonymous or identified performance feedback from party members with spam protection, Discord notifications, and a password-protected admin panel.

## Features

- **Star Ratings**: 5-category performance ratings (Mechanics, Damage, Teamwork, Communication, Overall)
- **Spam Protection**: IP-based rate limiting (configurable window)
- **Anonymous Submissions**: Optional anonymous feedback
- **Character Tracking**: Track player names, servers, jobs, and content type
- **Admin Panel**: Password-protected feedback management with live filtering and deletion
- **Discord Notifications**: Send formatted feedback summaries to Discord webhooks
- **Fully Configurable**: Environment variables for all settings, easy multi-instance deployment

## Quick Start

### Prerequisites

- **Rust** 1.70+ ([Install Rust](https://rustup.rs/))
- Basic command line knowledge

### 1. Clone and Build

```bash
git clone https://github.com/yourusername/FinalFeedback.git
cd FinalFeedback
cargo build --release
```

### 2. Configure Environment

```bash
cp .env.example .env
```

Edit `.env` with your settings. **Important**: Values with spaces must be quoted!

```env
# Server Configuration
HOST=127.0.0.1
PORT=8080
DATABASE_PATH=feedback.db

# Security - CHANGE THIS!
ADMIN_PASSWORD=your_secure_password_here

# Rate Limiting (minutes between submissions per IP)
RATE_LIMIT_MINUTES=30

# Player Configuration - MUST USE QUOTES FOR SPACES
PLAYER_NAME="Your Character Name"
PLAYER_SERVER=YourServer
PLAYER_DATACENTER=YourDatacenter

# Images (optional)
BANNER_IMAGE=/assets/banner.webp
PROFILE_IMAGE=/assets/profile.webp

# Tagline (optional)
TAGLINE="Ran content with me? Let me know how I did!"

# Discord (optional)
DISCORD_WEBHOOK_URL=
```

### 3. Run the Application

```bash
cargo run --release
```

Access the feedback form at `http://localhost:8080`

Access the admin panel at `http://localhost:8080/admin/panel` (use your `ADMIN_PASSWORD`)

## Configuration Reference

### Important: .env Format

**Values with spaces MUST be quoted:**
```env
CORRECT:
PLAYER_NAME="Violet Aerithil"

WRONG - Will fail:
PLAYER_NAME=Violet Aerithil
```

### Server Settings
- `HOST`: Bind address (default: `127.0.0.1`)
- `PORT`: Port number (default: `8080`)

### Database
- `DATABASE_PATH`: SQLite file path (default: `feedback.db`)
  - Auto-created if doesn't exist
  - Can be relative or absolute path

### Security
- `ADMIN_PASSWORD`: Admin panel password
  - **CRITICAL**: Change from default in production!
  - Use strong random password

### Rate Limiting
- `RATE_LIMIT_MINUTES`: Time window for spam protection (default: `30`)
  - Example: `RATE_LIMIT_MINUTES=60` = 1 submission per hour per IP

### Player Customization
- `PLAYER_NAME`: Character name (quote if spaces)
- `PLAYER_SERVER`: Server name
- `PLAYER_DATACENTER`: Datacenter name

### Images
- `BANNER_IMAGE`: Banner image path (default: `/assets/banner.webp`)
- `PROFILE_IMAGE`: Profile picture path (default: `/assets/profile.webp`)

### Tagline
- `TAGLINE`: Custom subtitle on the feedback form (default: `"Ran content with me? Let me know how I did!"`)
  - Can be any text to encourage feedback

### Discord Integration
- `DISCORD_WEBHOOK_URL`: Discord webhook URL for notifications
- Leave empty to disable
