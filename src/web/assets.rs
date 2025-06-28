// Static HTML/CSS/JS assets for the web terminal interface

pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rule Agents Terminal v2</title>
    <link rel="stylesheet" type="text/css" href="https://cdn.jsdelivr.net/npm/asciinema-player@3.7.0/dist/bundle/asciinema-player.css" />
    <style>
        body {
            margin: 0;
            padding: 0;
            background-color: #282a36;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            color: #f8f8f2;
            height: 100vh;
            overflow: hidden;
        }
        
        .container {
            height: 100vh;
            display: flex;
            flex-direction: column;
        }
        
        .header {
            background-color: #44475a;
            padding: 10px 20px;
            border-bottom: 1px solid #6272a4;
            font-size: 14px;
        }
        
        .title {
            color: #bd93f9;
            font-weight: bold;
        }
        
        .status {
            color: #50fa7b;
            margin-left: 20px;
        }
        
        .terminal {
            flex: 1;
            background-color: #282a36;
            padding: 20px;
            overflow: hidden;
            display: flex;
            flex-direction: column;
        }
        
        .terminal-container {
            flex: 1;
            background-color: #282a36;
            overflow: auto;
            border: none;
            width: 100%;
            min-height: 400px;
        }
        
        /* Override asciinema-player styles for better integration */
        .asciinema-player .asciinema-terminal {
            background-color: #282a36 !important;
            color: #f8f8f2 !important;
            font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', 'JetBrains Mono', 'Consolas', 'Monaco', 'Menlo', 'Ubuntu Mono', monospace !important;
            font-size: clamp(10px, 1.2vw, 14px) !important;
            font-variant-ligatures: none !important;
            font-feature-settings: "liga" 0 !important;
            font-variant-numeric: tabular-nums !important;
            font-kerning: none !important;
            letter-spacing: 0 !important;
            overflow: visible !important;
            line-height: 1.2 !important;
            text-rendering: optimizeSpeed !important;
        }
        
        .asciinema-player .asciinema-player-wrapper {
            background-color: transparent !important;
        }
        
        .asciinema-player {
            width: 100% !important;
            height: 100% !important;
            font-size: clamp(8px, 1.2vw, 14px) !important;
        }
        
        .asciinema-player .asciinema-terminal {
            max-width: none !important;
            max-height: none !important;
            padding: 10px !important;
        }
        
        .asciinema-player .asciinema-terminal .line {
            white-space: pre !important;
            overflow: visible !important;
            font-feature-settings: "liga" 0 !important;
            text-rendering: optimizeSpeed !important;
        }
        
        
        /* Hide scrollbars but keep scroll functionality */
        .terminal::-webkit-scrollbar,
        .terminal-container::-webkit-scrollbar,
        .asciinema-player::-webkit-scrollbar {
            width: 0px;
            background: transparent;
        }
        
        .terminal,
        .terminal-container,
        .asciinema-player {
            scrollbar-width: none; /* Firefox */
            -ms-overflow-style: none; /* Internet Explorer 10+ */
        }
        
        .input-area {
            background-color: #44475a;
            border-top: 1px solid #6272a4;
            padding: 8px 20px;
            display: flex;
            align-items: center;
        }
        
        .prompt {
            color: #bd93f9;
            margin-right: 8px;
            font-size: 12px;
        }
        
        #commandInput {
            flex: 1;
            background: transparent;
            border: none;
            color: #f8f8f2;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            font-size: 12px;
            outline: none;
        }
        
        .cursor {
            background-color: #f8f8f2;
            animation: blink 1s infinite;
        }
        
        @keyframes blink {
            0%, 50% { opacity: 1; }
            51%, 100% { opacity: 0; }
        }
        
        .connection-status {
            position: fixed;
            top: 10px;
            right: 10px;
            padding: 5px 10px;
            border-radius: 3px;
            font-size: 12px;
            font-weight: bold;
        }
        
        .connected {
            background-color: #50fa7b;
            color: #000;
        }
        
        .disconnected {
            background-color: #ff5555;
            color: #fff;
        }
        
        .reconnecting {
            background-color: #f1fa8c;
            color: #000;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <span class="title">Rule Agents Terminal</span>
            <span class="status">Connected to Agent</span>
        </div>
        
        <div class="terminal">
            <div id="terminal-container" class="terminal-container">
                <div id="asciinema-player"></div>
            </div>
        </div>
        
        <div class="input-area">
            <span class="prompt">$</span>
            <input type="text" id="commandInput" placeholder="Type commands here...">
        </div>
    </div>
    
    <div id="connectionStatus" class="connection-status disconnected">Disconnected</div>
    
    <script src="https://cdn.jsdelivr.net/npm/asciinema-player@3.7.0/dist/bundle/asciinema-player.min.js"></script>
    <script>
        class TerminalClient {
            constructor() {
                this.terminal = document.getElementById('asciinema-player');
                this.input = document.getElementById('commandInput');
                this.status = document.getElementById('connectionStatus');
                this.player = null;
                this.ws = null;
                this.cols = 80;  // Default, will be updated from config
                this.rows = 24;  // Default, will be updated from config
                this.setupEventListeners();
                this.loadConfig();
            }

            async loadConfig() {
                try {
                    const response = await fetch('/config');
                    const config = await response.json();
                    this.cols = config.cols;
                    this.rows = config.rows;
                    console.log('📐 Terminal config loaded:', { cols: this.cols, rows: this.rows });
                    
                    // Create player using ALiS protocol (direct WebSocket)
                    this.setupALiSPlayer();
                } catch (error) {
                    console.error('❌ Failed to load config, using defaults:', error);
                    this.setupALiSPlayer();
                }
            }
            
            setupALiSPlayer() {
                // Create WebSocket connection manually and feed to asciinema player
                const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                const wsUrl = `${protocol}//${window.location.host}/ws`;
                
                console.log('🔌 Connecting to WebSocket:', wsUrl);
                
                const ws = new WebSocket(wsUrl);
                let events = [];
                let header = null;
                
                ws.onopen = () => {
                    console.log('✅ WebSocket connected');
                    this.updateStatus('connected', 'Connected to Agent');
                };
                
                ws.onmessage = (event) => {
                    try {
                        const data = JSON.parse(event.data);
                        console.log('📨 Received HT-style event:', data);
                        
                        if (data.type === 'init') {
                            // This is the HT-style init event
                            console.log('📋 Received init event:', data);
                            this.createTerminalDisplay(data);
                            if (data.data) {
                                this.setTerminalContent(data.data);
                            }
                        } else if (data.type === 'output') {
                            // This is an HT-style output event
                            console.log('📝 Received output event:', data);
                            if (this.terminalOutput) {
                                this.appendContent(data.data);
                            }
                        } else if (data.type === 'resize') {
                            // Handle resize events
                            console.log('📐 Received resize event:', data);
                            this.handleResize(data.cols, data.rows);
                        }
                    } catch (e) {
                        console.error('❌ Failed to parse WebSocket message:', e, event.data);
                    }
                };
                
                ws.onclose = () => {
                    console.log('🔌 WebSocket disconnected');
                    this.updateStatus('disconnected', 'Disconnected');
                    setTimeout(() => this.setupALiSPlayer(), 3000);
                };
                
                ws.onerror = (error) => {
                    console.error('❌ WebSocket error:', error);
                    this.updateStatus('reconnecting', 'Reconnecting...');
                };
                
                this.ws = ws;
            }
            
            createTerminalDisplay(initData) {
                console.log('🎬 Creating HT-style terminal display:', initData);
                
                // Create a simple terminal display instead of asciinema player
                this.terminal.innerHTML = `
                    <div style="
                        font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', 'JetBrains Mono', 'Consolas', 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
                        font-size: 12px;
                        line-height: 1.2;
                        background-color: #282a36;
                        color: #f8f8f2;
                        padding: 10px;
                        white-space: pre-wrap;
                        overflow: auto;
                        height: 100%;
                        width: 100%;
                        word-wrap: break-word;
                    " id="terminal-output"></div>
                `;
                
                this.terminalOutput = document.getElementById('terminal-output');
                this.cols = initData.cols;
                this.rows = initData.rows;
                
                console.log('✅ HT-style terminal display created successfully');
            }
            
            setTerminalContent(content) {
                if (this.terminalOutput) {
                    // AVT has already processed the content properly, just convert ANSI to HTML
                    const cleanContent = this.convertAnsiToHtml(content);
                    this.terminalOutput.innerHTML = cleanContent;
                    this.terminalOutput.scrollTop = this.terminalOutput.scrollHeight;
                    console.log('📺 Set AVT-processed terminal content:', content.length, 'chars');
                }
            }
            
            processScreenContent(content) {
                // This mimics what a real terminal emulator would show
                // Remove excessive cursor movements and clear sequences but preserve colors
                let processed = content
                    // Remove excessive clear sequences 
                    .replace(/(\x1b\[2J\x1b\[H)+/g, '\x1b[2J\x1b[H')
                    // Remove excessive line clears but not all cursor movements
                    .replace(/(\x1b\[2K\x1b\[1A){3,}/g, '\x1b[2K\x1b[1A')
                    // Remove some excessive single-direction cursor movements
                    .replace(/(\x1b\[[0-9]*A){5,}/g, '') // Remove excessive up movements
                    .replace(/(\x1b\[[0-9]*B){5,}/g, '') // Remove excessive down movements
                    // Keep color codes and other formatting intact
                    // Clean up broken positioning at the end
                    .replace(/\x1b\[H([^]+)$/g, '$1'); // Remove positioning and keep content
                
                return processed;
            }
            
            handleResize(cols, rows) {
                this.cols = cols;
                this.rows = rows;
                console.log('📐 Terminal resized to:', cols, 'x', rows);
            }
            
            appendContent(content) {
                if (this.terminalOutput) {
                    // AVT handles screen clearing properly, so we just append
                    const cleanContent = this.convertAnsiToHtml(content);
                    this.terminalOutput.innerHTML += cleanContent;
                    this.terminalOutput.scrollTop = this.terminalOutput.scrollHeight;
                }
            }
            
            convertAnsiToHtml(text) {
                // Escape HTML first
                let html = text
                    .replace(/&/g, '&amp;')
                    .replace(/</g, '&lt;')
                    .replace(/>/g, '&gt;');
                
                // Convert common ANSI color codes to HTML
                const colorMap = {
                    '30': 'color: #282a36',     // black
                    '31': 'color: #ff5555',     // red
                    '32': 'color: #50fa7b',     // green
                    '33': 'color: #f1fa8c',     // yellow
                    '34': 'color: #bd93f9',     // blue
                    '35': 'color: #ff79c6',     // magenta
                    '36': 'color: #8be9fd',     // cyan
                    '37': 'color: #f8f8f2',     // white
                    '90': 'color: #6272a4',     // bright black (gray)
                    '91': 'color: #ff6e6e',     // bright red
                    '92': 'color: #69ff94',     // bright green
                    '93': 'color: #ffffa5',     // bright yellow
                    '94': 'color: #d6acff',     // bright blue
                    '95': 'color: #ff92df',     // bright magenta
                    '96': 'color: #a4ffff',     // bright cyan
                    '97': 'color: #ffffff'      // bright white
                };
                
                // Convert ANSI color codes
                html = html.replace(/\x1b\[([0-9;]*)m/g, (match, codes) => {
                    if (!codes) return '</span>';
                    
                    const codeList = codes.split(';');
                    let styles = [];
                    
                    for (const code of codeList) {
                        if (code === '0') {
                            return '</span>'; // Reset
                        } else if (code === '1') {
                            styles.push('font-weight: bold');
                        } else if (code === '2') {
                            styles.push('opacity: 0.7');
                        } else if (code === '7') {
                            styles.push('background-color: #f8f8f2; color: #282a36'); // Reverse
                        } else if (colorMap[code]) {
                            styles.push(colorMap[code]);
                        }
                    }
                    
                    // Handle 24-bit RGB colors separately
                    if (codes.includes('38;2')) {
                        const rgbMatch = codes.match(/38;2;(\d+);(\d+);(\d+)/);
                        if (rgbMatch) {
                            const [, r, g, b] = rgbMatch;
                            styles.push(`color: rgb(${r}, ${g}, ${b})`);
                        }
                    }
                    
                    // Handle 24-bit RGB background colors
                    if (codes.includes('48;2')) {
                        const rgbMatch = codes.match(/48;2;(\d+);(\d+);(\d+)/);
                        if (rgbMatch) {
                            const [, r, g, b] = rgbMatch;
                            styles.push(`background-color: rgb(${r}, ${g}, ${b})`);
                        }
                    }
                    
                    return styles.length > 0 ? `<span style="${styles.join('; ')}">` : '';
                });
                
                // Handle cursor movements and clear sequences
                html = html
                    .replace(/\x1b\[2J/g, '') // Clear screen
                    .replace(/\x1b\[3J/g, '') // Clear scrollback
                    .replace(/\x1b\[H/g, '')  // Move cursor to home
                    .replace(/\x1b\[[0-9]*A/g, '') // Move cursor up
                    .replace(/\x1b\[[0-9]*B/g, '') // Move cursor down
                    .replace(/\x1b\[[0-9]*C/g, '') // Move cursor right
                    .replace(/\x1b\[[0-9]*D/g, '') // Move cursor left
                    .replace(/\x1b\[[0-9]*G/g, '') // Move cursor to column
                    .replace(/\x1b\[[0-9]*K/g, '') // Clear line
                    .replace(/\x1b\[2K/g, '')      // Clear entire line
                    .replace(/\x1b\[\?[0-9]*[hl]/g, '') // Various terminal modes
                    .replace(/\x1b\[\?25[lh]/g, '') // Hide/show cursor
                    .replace(/\x1b\[\?2004[hl]/g, '') // Bracketed paste mode
                    .replace(/\r\n/g, '\n')
                    .replace(/\r/g, '\n');
                
                return html;
            }
            
            
            setupEventListeners() {
                this.input.addEventListener('keydown', (e) => {
                    if (e.key === 'Enter') {
                        e.preventDefault();
                        const command = this.input.value;
                        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                            console.log('📤 Sending command:', JSON.stringify(command));
                            this.ws.send(command + '\r');
                            this.input.value = '';
                        } else {
                            console.error('❌ WebSocket not connected');
                        }
                    }
                });
                
                // Focus input when clicking on terminal container
                const terminalContainer = document.getElementById('terminal-container');
                if (terminalContainer) {
                    terminalContainer.addEventListener('click', () => {
                        this.input.focus();
                    });
                }
                
                this.input.focus();
            }
            
            
            updateStatus(className, text) {
                this.status.className = `connection-status ${className}`;
                this.status.textContent = text;
            }
        }
        
        // Initialize terminal client when page loads
        document.addEventListener('DOMContentLoaded', () => {
            new TerminalClient();
        });
    </script>
</body>
</html>
"#;
