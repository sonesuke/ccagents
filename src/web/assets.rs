// Static HTML/CSS/JS assets for the web terminal interface

pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rule Agents Terminal</title>
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
            overflow-y: auto;
            white-space: pre-wrap;
            font-size: 14px;
            line-height: 1.4;
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
        
        <div id="terminal" class="terminal"></div>
        
        <div class="input-area">
            <span class="prompt">$</span>
            <input type="text" id="commandInput" placeholder="Type commands here...">
        </div>
    </div>
    
    <div id="connectionStatus" class="connection-status disconnected">Disconnected</div>
    
    <script>
        class TerminalClient {
            constructor() {
                this.ws = null;
                this.terminal = document.getElementById('terminal');
                this.input = document.getElementById('commandInput');
                this.status = document.getElementById('connectionStatus');
                this.setupEventListeners();
                this.connect();
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
                        }
                    }
                });
                
                // Focus input when clicking on terminal
                this.terminal.addEventListener('click', () => {
                    this.input.focus();
                });
                
                // Auto-focus input on page load
                this.input.focus();
            }
            
            appendToTerminal(text) {
                this.terminal.textContent = text;
                this.scrollToBottom();
            }
            
            scrollToBottom() {
                this.terminal.scrollTop = this.terminal.scrollHeight;
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
