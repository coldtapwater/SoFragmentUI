import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import React, { useState, useRef, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Message, MessageBoxProps } from '../types/chat';

const MessageBox: React.FC<MessageBoxProps> = ({ isGenerating, children, messageDuration, isAiMessage }) => {
  return (
    <motion.div className="relative m-3">
      <div className="p-4 mx-2 font-serif text-white">
        {children}
      </div>
      {isAiMessage && (
        <motion.div
          className="absolute inset-0"
          initial={{ pathLength: 0 }}
        >
          <svg className="absolute inset-0 w-full h-full">
            {/* Top line */}
            <motion.line
              x1="0" y1="0" x2="100%" y2="0"
              stroke="white"
              strokeWidth="1"
              initial={{ pathLength: 0, opacity: 0 }}
              animate={{ 
                pathLength: 1, 
                opacity: isGenerating ? 0.3 : 1 
              }}
              transition={{ 
                pathLength: { duration: isGenerating ? (messageDuration || 0) * 0.25 : 0 },
                opacity: { duration: 0.3 }
              }}
            />
            {/* Right line */}
            <motion.line
              x1="100%" y1="0" x2="100%" y2="100%"
              stroke="white"
              strokeWidth="1"
              initial={{ pathLength: 0, opacity: 0 }}
              animate={{ 
                pathLength: 1, 
                opacity: isGenerating ? 0.3 : 1 
              }}
              transition={{ 
                pathLength: { duration: isGenerating ? (messageDuration || 0) * 0.25 : 0, delay: isGenerating ? (messageDuration || 0) * 0.25 : 0 },
                opacity: { duration: 0.3 }
              }}
            />
            {/* Bottom line */}
            <motion.line
              x1="100%" y1="100%" x2="0" y2="100%"
              stroke="white"
              strokeWidth="1"
              initial={{ pathLength: 0, opacity: 0 }}
              animate={{ 
                pathLength: 1, 
                opacity: isGenerating ? 0.3 : 1 
              }}
              transition={{ 
                pathLength: { duration: isGenerating ? (messageDuration || 0) * 0.25 : 0, delay: isGenerating ? (messageDuration || 0) * 0.5 : 0 },
                opacity: { duration: 0.3 }
              }}
            />
            {/* Left line */}
            <motion.line
              x1="0" y1="100%" x2="0" y2="0"
              stroke="white"
              strokeWidth="1"
              initial={{ pathLength: 0, opacity: 0 }}
              animate={{ 
                pathLength: 1, 
                opacity: isGenerating ? 0.3 : 1 
              }}
              transition={{ 
                pathLength: { duration: isGenerating ? (messageDuration || 0) * 0.25 : 0, delay: isGenerating ? (messageDuration || 0) * 0.75 : 0 },
                opacity: { duration: 0.3 }
              }}
            />
          </svg>
        </motion.div>
      )}
    </motion.div>
  );
};

const ChatInterface: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputValue, setInputValue] = useState<string>('');
  const [isGenerating, setIsGenerating] = useState<boolean>(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = (): void => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  useEffect(() => {
    // Set up event listener for streaming responses
    const unlisten = listen<string>('chat-response', (event) => {
      setMessages(prev => prev.map(msg => {
        if (msg.id === prev[prev.length - 1].id && msg.role === 'assistant') {
          return {
            ...msg,
            content: msg.content + event.payload
          };
        }
        return msg;
      }));
    });

    return () => {
      unlisten.then(f => f()); // Cleanup listener on unmount
    };
  }, []);

  const sendMessage = async (content: string): Promise<void> => {
    setIsGenerating(true);
    
    try {
      const aiMessage: Message = { 
        role: 'assistant',
        content: '',
        id: Date.now(),
        duration: messages.length * 0.9,
        isAiMessage: true
      };
      
      setMessages(prevMessages => [...prevMessages, aiMessage]);

      await invoke('chat_stream', { 
        message: content 
      });
    } catch (error) {
      console.error('Error getting response:', error);
      setMessages(prev => prev.map(msg => 
        msg.id === prev[prev.length - 1].id 
          ? { ...msg, content: "Sorry, I encountered an error. Please try again." }
          : msg
      ));
    } finally {
      setIsGenerating(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      const trimmedInput = inputValue.trim();
      
      if (trimmedInput) {
        setInputValue('');
        
        const userMessage: Message = {
          role: 'user',
          content: trimmedInput,
          id: Date.now(),
          isAiMessage: false
        };
        
        setMessages(prevMessages => [...prevMessages, userMessage]);
        sendMessage(trimmedInput);
      }
    }
  };

  return (
    <div className="h-screen flex flex-col bg-gray-800 bg-opacity-50 backdrop-blur-sm">
      <div className="flex-1 overflow-y-auto p-4">
        <div className="max-w-3xl mx-auto">
          {messages.map((message) => (
            <motion.div
              key={message.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="mb-4"
            >
              <MessageBox 
                isGenerating={isGenerating && message.isAiMessage} 
                messageDuration={message.duration}
                isAiMessage={message.isAiMessage}
              >
                {message.content}
              </MessageBox>
            </motion.div>
          ))}
          <div ref={messagesEndRef} />
        </div>
      </div>
      
      <div className="p-4">
        <div className="max-w-3xl mx-auto">
          <textarea
            ref={inputRef}
            value={inputValue}
            onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a message..."
            className="w-full p-4 bg-gray-700 bg-opacity-50 rounded-full text-white resize-none"
            rows={1}
            style={{
              minHeight: '2.5rem',
              maxHeight: '10rem',
              overflow: 'auto'
            }}
          />
        </div>
      </div>
    </div>
  );
};

export default ChatInterface;