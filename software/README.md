# Software
The desktop app that connects to the JukeBox to control its RGB and display, written in Rust for Windows and Linux.

# Building
It's as simple as running `cargo build --release`.

## Discord support
Discord, currently, will not provide support to new projects using their RPC protocol. Because of this, JukeBox Desktop will not support Discord out of the box, despite having functionality for it built in. To use the Discord functionality in JukeBox Desktop, you must do the following:
1. Go to https://discord.com/developers/applications/ and log in with your account.
2. Create a new application and name it JukeBoxDesktop.
3. Go to the OAuth2 settings tab.
4. Copy the client ID and client secret down to a safe location.
5. Set the redirect to `https://localhost:61961`. Save your changes.
6. Go to the App Testers settings tab.
7. Add your Discord account as a tester. An email will be sent to your account email, go to your inbox and accept the invite.
8. Build with the following command: `DISCORD_CLIENT_ID="<PUT_CLIENT_ID_HERE>" DISCORD_CLIENT_SECRET="<PUT_CLIENT_SECRET_HERE>" cargo run --features "discord" --release`.
