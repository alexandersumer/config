# ChatGPT and Codex

Discover the browser tools exposed to this chat, including deferred tools. Follow the selected tool's first-call and browser selection instructions, then use the documentation for the handles it returns.

For an explicitly requested Chrome profile, match the extension and profile in the current inventory. A profile label can be absent even when the extension is available. Use the extension instance ID from a previously verified binding to recognize that profile when the label is missing. Use the current browser and tab IDs from discovery, since they can change after reconnecting. If the profile cannot be identified, ask for the browser or tab mention that identifies it.

Keep a working browser binding when a tab goes stale. Use the tool's browser troubleshooting guidance and obtain a fresh tab in the same browser. If the extension is missing or the tool reports a disconnect, refresh the inventory before asking the user to reconnect. If it returns, bind the matching extension and continue. Ask for help when a required step remains blocked after the documented recovery.

A slow navigation or page observation can exceed the tool's default deadline. Use a bounded longer per-call timeout when supported and justified by observed latency. If the tool reports a kernel reset, follow its first-call instructions again and recover the browser and tab from fresh state. Check whether the timed-out operation already took effect before retrying. Follow the tool's own recovery guidance for site-access denials and extension limits.
