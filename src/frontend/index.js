document.addEventListener('DOMContentLoaded', () => {
    const statusDot = document.getElementById('status-dot');
    const statusText = document.getElementById('status-text');
    const chatMessages = document.getElementById('chat-messages');
    const promptInput = document.getElementById('prompt-input');
    const btnCancel = document.getElementById('btn-cancel');
    const btnSend = document.getElementById('btn-send');
    const btnAttach = document.getElementById('btn-attach');
    const fileInput = document.getElementById('file-input');
    const attachmentPreviewContainer = document.getElementById('attachment-preview-container');
    const attachmentPreview = document.getElementById('attachment-preview');
    const btnRemoveAttachment = document.getElementById('btn-remove-attachment');
    const btnHuman = document.getElementById('btn-human');

    let sessionId = null;
    let ws = null;
    let sessionStatus = 'ready';
    let currentSSEEvent = null;
    let currentModelMessageContent = null;
    let currentModelMessageText = '';
    let selectedAttachment = null;

    // Configure marked.js for markdown rendering
    if (typeof marked !== 'undefined') {
        marked.setOptions({
            breaks: true,
            gfm: true,
            highlight: function(code, lang) {
                if (typeof hljs !== 'undefined' && lang && hljs.getLanguage(lang)) {
                    try {
                        return hljs.highlight(code, { language: lang }).value;
                    } catch (e) { /* fall through */ }
                }
                // Auto-detect for unknown languages
                if (typeof hljs !== 'undefined') {
                    try {
                        return hljs.highlightAuto(code).value;
                    } catch (e) { /* fall through */ }
                }
                return code;
            }
        });
    }

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

    // Transfer to Human button event
    if (btnHuman) {
        btnHuman.addEventListener('click', transferToHuman);
    }

    // Attachment events
    btnAttach.addEventListener('click', () => {
        fileInput.click();
    });

    fileInput.addEventListener('change', (e) => {
        const file = e.target.files[0];
        if (!file) return;

        if (file.size > 5 * 1024 * 1024) {
            alert("Image file size must be less than 5MB.");
            fileInput.value = '';
            return;
        }

        const reader = new FileReader();
        reader.onload = (event) => {
            const base64Data = event.target.result;
            selectedAttachment = {
                mime_type: file.type,
                data: base64Data
            };
            attachmentPreview.src = base64Data;
            attachmentPreviewContainer.style.display = 'inline-flex';
        };
        reader.readAsDataURL(file);
    });

    btnRemoveAttachment.addEventListener('click', () => {
        clearAttachment();
    });

    function clearAttachment() {
        selectedAttachment = null;
        fileInput.value = '';
        attachmentPreview.src = '';
        attachmentPreviewContainer.style.display = 'none';
    }

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
            sessionStatus = data.status || 'ready';
            console.log('Session initialized:', sessionId, 'Status:', sessionStatus);
            
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
                    sessionStatus = 'ready';
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
                } else if (data.type === 'session.status') {
                    sessionStatus = data.status;
                    if (data.status === 'human') {
                        updateStatus('connected', 'Human Operator Support');
                    } else if (data.status === 'ready') {
                        updateStatus('connected', 'Connected');
                    }
                } else if (data.type === 'operator.message') {
                    appendOperatorMessage(data.text, data.attachments);
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

        const attachments = selectedAttachment ? [selectedAttachment] : null;

        // Display user message in chat
        appendUserMessage(content, attachments);
        
        // Clear input and lock UI
        promptInput.value = '';
        promptInput.style.height = 'auto';
        clearAttachment();

        if (sessionStatus === 'human') {
            try {
                const response = await fetch(`/sessions/${sessionId}/messages`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({ content, attachments })
                });

                if (response.status !== 202) {
                    const errorText = await response.text();
                    appendSystemMessage(`Error: ${errorText}`);
                }
            } catch (error) {
                console.error('Failed to send message:', error);
                appendSystemMessage(`Network Error: ${error.message}`);
            }
            return;
        }

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
                body: JSON.stringify({ content, attachments })
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

    // Transfer session to human
    async function transferToHuman() {
        if (!sessionId) return;
        disableInputs();
        try {
            console.log('Sending transfer-to-human request for session:', sessionId);
            const response = await fetch(`/sessions/${sessionId}/human`, {
                method: 'POST'
            });

            if (response.ok) {
                sessionStatus = 'human';
                appendSystemMessage('已為您轉接人工客服。之後您的提問將由後台人工回覆。');
                enableInputs();
                updateStatus('connected', 'Human Operator Support');
            } else {
                console.error('Failed to transfer to human, status:', response.status);
                enableInputs();
            }
        } catch (error) {
            console.error('Error transferring to human:', error);
            enableInputs();
        }
    }

    // Create a new model response bubble in chat
    function createModelMessageElement() {
        const messageDiv = document.createElement('div');
        messageDiv.className = 'message model-message';
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content markdown-body';
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
    function appendUserMessage(text, attachments) {
        const messageDiv = document.createElement('div');
        messageDiv.className = 'message user-message';
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content';
        
        if (attachments && attachments.length > 0) {
            attachments.forEach(att => {
                if (att.mime_type.startsWith('image/')) {
                    const img = document.createElement('img');
                    img.src = att.data;
                    img.alt = 'Uploaded Image';
                    img.style.maxWidth = '100%';
                    img.style.borderRadius = '8px';
                    img.style.marginBottom = '8px';
                    img.style.display = 'block';
                    contentDiv.appendChild(img);
                }
            });
        }
        
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

    // Append operator (model) message bubble
    function appendOperatorMessage(text, attachments) {
        const messageDiv = document.createElement('div');
        messageDiv.className = 'message model-message';
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content markdown-body';
        
        if (attachments && attachments.length > 0) {
            attachments.forEach(att => {
                if (att.mime_type.startsWith('image/')) {
                    const img = document.createElement('img');
                    img.src = att.data;
                    img.alt = 'Operator Image';
                    img.style.maxWidth = '100%';
                    img.style.borderRadius = '8px';
                    img.style.marginBottom = '8px';
                    img.style.display = 'block';
                    contentDiv.appendChild(img);
                }
            });
        }
        
        const textDiv = document.createElement('div');
        textDiv.innerHTML = formatMarkdown(text);
        contentDiv.appendChild(textDiv);
        
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
        if (btnAttach) btnAttach.disabled = false;
        if (btnHuman) btnHuman.disabled = false;
        promptInput.focus();
    }

    function disableInputs() {
        promptInput.disabled = true;
        btnSend.disabled = true;
        btnCancel.disabled = true;
        if (btnAttach) btnAttach.disabled = true;
        if (btnHuman) btnHuman.disabled = true;
    }

    function lockInputsForStreaming() {
        promptInput.disabled = true;
        btnSend.disabled = true;
        btnCancel.disabled = false;
        if (btnAttach) btnAttach.disabled = true;
        if (btnHuman) btnHuman.disabled = true;
    }

    function scrollToBottom() {
        chatMessages.scrollTop = chatMessages.scrollHeight;
    }

    // Markdown rendering using marked.js with fallback
    function formatMarkdown(text) {
        if (!text) return '';

        // Use marked.js if available
        if (typeof marked !== 'undefined') {
            try {
                return marked.parse(text);
            } catch (e) {
                console.error('marked.parse() error:', e);
            }
        }

        // Fallback: basic markdown rendering
        let html = text
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;');

        // Images
        html = html.replace(/!\[([^\]]*?)\]\(([^)]+?)\)/g, '<img src="$2" alt="$1" />');
        // Bold
        html = html.replace(/\*\*([^*]+?)\*\*/g, '<strong>$1</strong>');
        // Italic
        html = html.replace(/\*([^*]+?)\*/g, '<em>$1</em>');
        // Code blocks
        html = html.replace(/```(?:[a-zA-Z0-9]+)?\n([\s\S]*?)(?:```|$)/g, '<pre><code>$1</code></pre>');
        // Inline code
        html = html.replace(/`([^`\n]+)`/g, '<code>$1</code>');
        // Headers
        html = html.replace(/^### (.+)$/gm, '<h3>$1</h3>');
        html = html.replace(/^## (.+)$/gm, '<h2>$1</h2>');
        html = html.replace(/^# (.+)$/gm, '<h1>$1</h1>');

        // Paragraphs
        const parts = html.split('\n\n');
        return parts.map(part => {
            const trimmed = part.trim();
            if (!trimmed || trimmed.startsWith('<pre>') || trimmed.startsWith('<h')) return trimmed;
            return `<p>${trimmed.replace(/\n/g, '<br>')}</p>`;
        }).join('');
    }
});
