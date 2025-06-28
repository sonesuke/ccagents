// Static HTML/CSS/JS assets for the web terminal interface

pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rule Agents Terminal</title>
    <link rel="stylesheet" type="text/css" href="https://cdn.jsdelivr.net/npm/asciinema-player@3.7.0/dist/bundle/asciinema-player.css" />
    <style>
        body {
            margin: 0;
            padding: 0;
            background-color: #000;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            color: #fff;
            height: 100vh;
            overflow: hidden;
        }
        
        .container {
            height: 100vh;
            display: flex;
            flex-direction: column;
        }
        
        .header {
            background-color: #1a1a1a;
            padding: 10px 20px;
            border-bottom: 1px solid #333;
            font-size: 14px;
        }
        
        .title {
            color: #4a9eff;
            font-weight: bold;
        }
        
        .status {
            color: #50fa7b;
            margin-left: 20px;
        }
        
        .terminal {
            flex: 1;
            background-color: #000;
            padding: 20px;
            overflow: hidden;
            display: flex;
            flex-direction: column;
        }
        
        .terminal-container {
            flex: 1;
            background-color: #000;
            border-radius: 6px;
            overflow: hidden;
            border: 1px solid #333;
        }
        
        /* Override asciinema-player styles for better integration */
        .asciinema-player .asciinema-terminal {
            background-color: #000 !important;
            color: #fff !important;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace !important;
            font-size: 14px !important;
            line-height: 1.4 !important;
        }
        
        .asciinema-player .asciinema-player-wrapper {
            background-color: transparent !important;
        }
        
        .asciinema-player .asciinema-terminal .line {
            height: auto !important;
        }
        
        .terminal::-webkit-scrollbar {
            width: 8px;
        }
        
        .terminal::-webkit-scrollbar-track {
            background: #1a1a1a;
        }
        
        .terminal::-webkit-scrollbar-thumb {
            background: #444;
            border-radius: 4px;
        }
        
        .terminal::-webkit-scrollbar-thumb:hover {
            background: #666;
        }
        
        .input-area {
            background-color: #1a1a1a;
            border-top: 1px solid #333;
            padding: 10px 20px;
            display: flex;
            align-items: center;
        }
        
        .prompt {
            color: #4a9eff;
            margin-right: 10px;
        }
        
        #commandInput {
            flex: 1;
            background: transparent;
            border: none;
            color: #fff;
            font-family: inherit;
            font-size: 14px;
            outline: none;
        }
        
        .cursor {
            background-color: #fff;
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
                this.ws = null;
                this.terminal = document.getElementById('asciinema-player');
                this.input = document.getElementById('commandInput');
                this.status = document.getElementById('connectionStatus');
                this.player = null;
                this.terminalData = [];
                this.startTime = Date.now();
                this.cols = 80;
                this.rows = 24;
                this.setupPlayer();
                this.setupEventListeners();
                this.connect();
            }
            
            setupPlayer() {
                // Initialize asciinema player with empty data
                const initialData = {
                    version: 2,
                    width: this.cols,
                    height: this.rows,
                    timestamp: Math.floor(this.startTime / 1000)
                };
                
                this.player = AsciinemaPlayer.create(
                    { 
                        data: [initialData, [0, "o", "Rule Agents Terminal Ready\r\n$ "]]
                    },
                    this.terminal,
                    {
                        autoPlay: true,
                        loop: false,
                        controls: false,
                        terminalFontSize: '14px',
                        terminalFontFamily: 'Monaco, Menlo, Ubuntu Mono, monospace',
                        fit: 'width',
                        theme: 'asciinema'
                    }
                );
            }
            
            updatePlayerData() {
                // Debounce updates to avoid too frequent recreations
                if (this.updateTimeout) {
                    clearTimeout(this.updateTimeout);
                }
                
                this.updateTimeout = setTimeout(() => {
                    if (this.player && this.terminalData.length > 0) {
                        const header = {
                            version: 2,
                            width: this.cols,
                            height: this.rows,
                            timestamp: Math.floor(this.startTime / 1000)
                        };
                        
                        const fullData = {
                            data: [header, ...this.terminalData]
                        };
                        
                        // Recreate player with new data
                        if (this.player.dispose) {
                            this.player.dispose();
                        }
                        this.player = AsciinemaPlayer.create(
                            fullData,
                            this.terminal,
                            {
                                autoPlay: true,
                                loop: false,
                                controls: false,
                                terminalFontSize: '14px',
                                terminalFontFamily: 'Monaco, Menlo, Ubuntu Mono, monospace',
                                fit: 'width',
                                theme: 'asciinema'
                            }
                        );
                    }
                }, 100); // Debounce by 100ms
            }
            
            connect() {
                const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                const wsUrl = `${protocol}//${window.location.host}/ws`;
                
                this.updateStatus('reconnecting', 'Connecting...');
                
                this.ws = new WebSocket(wsUrl);
                
                this.ws.onopen = () => {
                    this.updateStatus('connected', 'Connected');
                    console.log('WebSocket connected');
                };
                
                this.ws.onmessage = (event) => {
                    this.appendToTerminal(event.data);
                };
                
                this.ws.onclose = () => {
                    this.updateStatus('disconnected', 'Disconnected');
                    console.log('WebSocket disconnected');
                    // Attempt to reconnect after 3 seconds
                    setTimeout(() => this.connect(), 3000);
                };
                
                this.ws.onerror = (error) => {
                    console.error('WebSocket error:', error);
                    this.updateStatus('disconnected', 'Connection Error');
                };
            }
            
            setupEventListeners() {
                this.input.addEventListener('keydown', (e) => {
                    if (e.key === 'Enter') {
                        const command = this.input.value;
                        if (command.trim() && this.ws && this.ws.readyState === WebSocket.OPEN) {
                            this.ws.send(command + '\n');
                            this.input.value = '';
                            
                            // Add input to terminal data for asciinema
                            const timestamp = (Date.now() - this.startTime) / 1000;
                            this.terminalData.push([timestamp, "o", command + "\r\n"]);
                            this.updatePlayerData();
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
                
                // Auto-focus input on page load
                this.input.focus();
            }
            
            appendToTerminal(text) {
                // Add terminal output to asciinema data
                const timestamp = (Date.now() - this.startTime) / 1000;
                this.terminalData.push([timestamp, "o", text]);
                this.updatePlayerData();
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
