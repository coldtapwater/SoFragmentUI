export interface Message {
    role: 'user' | 'assistant';
    content: string;
    id: number;
    duration?: number;
    isAiMessage: boolean;
  }
  
  export interface MessageBoxProps {
    isGenerating: boolean;
    children: React.ReactNode;
    messageDuration?: number;
    isAiMessage: boolean;
  }