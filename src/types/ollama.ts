export interface ChatMessage {
    role: 'user' | 'assistant';
    content: string;
  }
  
  export interface ChatRequest {
    model: string;
    messages: ChatMessage[];
    stream: boolean;
  }
  
  export interface ChatResponse {
    model: string;
    message: ChatMessage;
    done: boolean;
  }