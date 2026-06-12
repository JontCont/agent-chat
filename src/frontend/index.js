document.addEventListener('DOMContentLoaded', () => {
    const statusDot = document.getElementById('status-dot');
    const statusText = document.getElementById('status-text');
    const chatMessages = document.getElementById('chat-messages');
    const promptInput = document.getElementById('prompt-input');
    const btnCancel = document.getElementById('btn-cancel');
    const btnSend = document.getElementById('btn-send');

    let sessionId = null;
    let ws = null;
    let currentSSEEvent = null;
    let currentModelMessageContent = null;
    let currentModelMessageText = '';

    // Initialize the application
    initSession();

    // Textarea auto-resize
    promptInput.addEventListener('input', () => {
        promptInput.style.height = 'auto';
        promptInput.style.height = `${promptInput.scrollHeight}px`;
    });

    // Send button event
    btnSend.addEventListener('click', sendMessage);

    // Enter key event in textarea
    promptInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    });

    // Cancel button event
    btnCancel.addEventListener('click', cancelMessage);

    // Initialize session
    async function initSession() {
        updateStatus('connecting', 'Initializing Session...');
        appendSystemMessage('System is initializing a new session...');

        try {
            const response = await fetch('/sessions', {
                method: 'POST'
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const data = await response.json();
            sessionId = data.id;
            console.log('Session initialized:', sessionId);
            
            appendSystemMessage('Session established. Connecting to server stream...');
            connectWebSocket(sessionId);
        } catch (error) {
            console.error('Failed to initialize session:', error);
            updateStatus('disconnected', 'Disconnected');
            appendSystemMessage('Failed to initialize session. Please refresh the page to try again.');
        }
    }

    // Connect to WebSocket
    function connectWebSocket(id) {
        const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${wsProtocol}//${window.location.host}/ws/${id}`;

        console.log('Connecting to WebSocket:', wsUrl);
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
            console.log('WebSocket connection established.');
            updateStatus('connected', 'Connected');
            appendSystemMessage('Connected. Ready to chat!');
            enableInputs();
        };

        ws.onmessage = (event) => {
            const messageStr = event.data;
            console.log('WebSocket received raw message:', messageStr);

            // Handle Server-Sent Events wrapped in WebSocket
            if (messageStr.startsWith('event: ')) {
                currentSSEEvent = messageStr.substring(7).trim();
                return;
            }

            if (messageStr.startsWith('data: ')) {
                const dataStr = messageStr.substring(6).trim();
                try {
                    const payload = JSON.parse(dataStr);
                    handleSSEPayload(currentSSEEvent, payload);
                } catch (e) {
                    console.error('Failed to parse SSE JSON data:', e);
                }
                return;
            }

            // Handle direct JSON notifications
            try {
                const data = JSON.parse(messageStr);
                if (data.type === 'session.ready') {
                    finalizeStream();
                } else if (data.type === 'session.error') {
                    showErrorInChat(data.error || 'An unexpected execution error occurred.');
                    finalizeStream();
                } else if (data.type === 'delta' || (data.event === 'delta' && data.text)) {
                    appendDelta(data.text);
                } else if (data.event === 'done' || data.type === 'done') {
                    finalizeStream();
                } else if (data.event === 'error' || data.type === 'error') {
                    showErrorInChat(data.error || data.text || 'Error streaming response.');
                    finalizeStream();
                }
            } catch (e) {
                console.log('WebSocket received non-JSON plain text:', messageStr);
            }
        };

        ws.onclose = (event) => {
            console.log('WebSocket connection closed.', event);
            updateStatus('disconnected', 'Disconnected');
            appendSystemMessage('Connection lost. Please refresh the page to reconnect.');
            disableInputs();
        };

        ws.onerror = (error) => {
            console.error('WebSocket error occurred:', error);
            updateStatus('disconnected', 'Connection Error');
        };
    }

    // Process SSE event payload
    function handleSSEPayload(event, payload) {
        if (event === 'delta') {
            if (payload.text) {
                appendDelta(payload.text);
            }
        } else if (event === 'done') {
            // Streaming finished successfully
            finalizeStream();
        } else if (event === 'error') {
            // Streaming encountered an error
            showErrorInChat(payload.error || payload.text || 'Error streaming response.');
            finalizeStream();
        }
    }

    // Append streaming text delta
    function appendDelta(text) {
        if (!currentModelMessageContent) {
            // Create a new model response container if none exists
            createModelMessageElement();
        }

        currentModelMessageText += text;
        currentModelMessageContent.innerHTML = formatMarkdown(currentModelMessageText);
        scrollToBottom();
    }

    // Finalize active stream
    function finalizeStream() {
        currentModelMessageContent = null;
        currentModelMessageText = '';
        currentSSEEvent = null;
        enableInputs();
    }

    // Send a message to backend
    async function sendMessage() {
        const content = promptInput.value.trim();
        if (!content || !sessionId) return;

        // Display user message in chat
        appendUserMessage(content);
        
        // Clear input and lock UI
        promptInput.value = '';
        promptInput.style.height = 'auto';
        lockInputsForStreaming();

        // Create empty model response bubble
        createModelMessageElement();
        scrollToBottom();

        try {
            const response = await fetch(`/sessions/${sessionId}/messages`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ content })
            });

            if (response.status === 202) {
                console.log('Message submission accepted (202). Awaiting stream...');
            } else {
                const errorText = await response.text();
                showErrorInChat(`Server error (${response.status}): ${errorText}`);
                finalizeStream();
            }
        } catch (error) {
            console.error('Failed to send message:', error);
            showErrorInChat(`Network error: ${error.message}`);
            finalizeStream();
        }
    }

    // Cancel current execution
    async function cancelMessage() {
        if (!sessionId) return;

        try {
            console.log('Sending cancel request for session:', sessionId);
            const response = await fetch(`/sessions/${sessionId}/cancel`, {
                method: 'POST'
            });

            if (response.ok) {
                appendSystemMessage('Cancellation request sent.');
            } else {
                console.error('Failed to cancel run, status:', response.status);
            }
        } catch (error) {
            console.error('Error during cancellation:', error);
        }
    }

    // Create a new model response bubble in chat
    function createModelMessageElement() {
        const messageDiv = document.createElement('div');
        messageDiv.className = 'message model-message';
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content';
        contentDiv.innerHTML = '<p><em>Thinking...</em></p>';

        messageDiv.appendChild(contentDiv);
        chatMessages.appendChild(messageDiv);
        
        currentModelMessageContent = contentDiv;
        currentModelMessageText = '';
    }

    // Display error message inside the current model response bubble
    function showErrorInChat(errorMessage) {
        if (!currentModelMessageContent) {
            createModelMessageElement();
        }
        currentModelMessageContent.innerHTML = `<p class="stream-error" style="color: #ff3b30; font-weight: bold;">Error: ${errorMessage}</p>`;
        scrollToBottom();
    }

    // Append user message
    function appendUserMessage(text) {
        const messageDiv = document.createElement('div');
        messageDiv.className = 'message user-message';
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content';
        
        // Escape HTML for user input
        const p = document.createElement('p');
        p.textContent = text;
        
        contentDiv.appendChild(p);
        messageDiv.appendChild(contentDiv);
        chatMessages.appendChild(messageDiv);
        scrollToBottom();
    }

    // Append system message
    function appendSystemMessage(text) {
        const messageDiv = document.createElement('div');
        messageDiv.className = 'message system-message';
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content';
        
        const p = document.createElement('p');
        p.textContent = text;
        
        contentDiv.appendChild(p);
        messageDiv.appendChild(contentDiv);
        chatMessages.appendChild(messageDiv);
        scrollToBottom();
    }

    // Update status indicator
    function updateStatus(state, text) {
        statusDot.className = `status-dot ${state}`;
        statusText.textContent = text;
    }

    // Helper functions for lock states
    function enableInputs() {
        promptInput.disabled = false;
        btnSend.disabled = false;
        btnCancel.disabled = true;
        promptInput.focus();
    }

    function disableInputs() {
        promptInput.disabled = true;
        btnSend.disabled = true;
        btnCancel.disabled = true;
    }

    function lockInputsForStreaming() {
        promptInput.disabled = true;
        btnSend.disabled = true;
        btnCancel.disabled = false;
    }

    function scrollToBottom() {
        chatMessages.scrollTop = chatMessages.scrollHeight;
    }

    // Basic markdown parsing to HTML
    function formatMarkdown(text) {
        if (!text) return '';

        // Escape HTML
        let html = text
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;');

        // Markdown Images: ![alt](url)
        html = html.replace(/!\[([^\]]*?)\]\(([^)]+?)\)/g, '<img src="$2" alt="$1" class="chat-image" />');

        // Code blocks: ```lang\ncode\n```
        html = html.replace(/```(?:[a-zA-Z0-9]+)?\n([\s\S]*?)(?:```|$)/g, '<pre><code>$1</code></pre>');

        // Inline code: `code`
        html = html.replace(/`([^`\n]+)`/g, '<code>$1</code>');

        // Split by double line breaks for paragraph breaks
        const parts = html.split('\n\n');
        const processedParts = parts.map(part => {
            const trimmed = part.trim();
            if (trimmed.startsWith('<pre>')) {
                return trimmed;
            }
            // Replace single newlines with <br>
            const inner = trimmed.replace(/\n/g, '<br>');
            return trimmed ? `<p>${inner}</p>` : '';
        });

        return processedParts.join('');
    }
});
