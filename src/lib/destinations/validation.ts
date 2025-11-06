// Destination validation utilities
export interface ValidationErrors {
  [field: string]: string;
}

interface DestinationConfig {
  type: string;
  [key: string]: any;
}

interface Destination {
  id: string;
  name: string;
  config: DestinationConfig;
  template?: string | null;
}

// Validate path format
const isValidPath = (path: string): boolean => {
  // Allow empty for validation to catch it
  if (!path) return true;

  // Basic path validation - allow ~ for home, / or \ for separators
  const pathRegex = /^(~?[/\\]?[\w\s\-._/\\]+|~)$/;
  return pathRegex.test(path);
};

// Validate chat ID format (Telegram)
const isValidChatId = (chatId: string): boolean => {
  // Allow empty for validation to catch it
  if (!chatId) return true;

  // Telegram chat IDs can be numeric or start with @ for channels
  const chatIdRegex = /^(@[\w]+|-?\d+)$/;
  return chatIdRegex.test(chatId);
};

// Check if filename pattern has valid tokens
const hasValidTokens = (pattern: string): boolean => {
  // Must include at least one token
  const tokens = ['{timestamp}', '{date}', '{time}', '{model_name}', '{workflow_name}'];
  return tokens.some(token => pattern.includes(token));
};

// Main validation function
export const validateDestination = (dest: Destination): ValidationErrors => {
  const errors: ValidationErrors = {};

  // Common validation
  if (!dest.name?.trim()) {
    errors.name = "Name is required";
  }

  // Type-specific validation
  switch (dest.config.type) {
    case "file_system":
      // Path validation
      if (!dest.config.path?.trim()) {
        errors.path = "Path is required";
      } else if (!isValidPath(dest.config.path)) {
        errors.path = "Invalid path format";
      }

      // Filename pattern validation
      if (!dest.config.filename_pattern?.trim()) {
        errors.filename_pattern = "Filename pattern is required";
      } else if (!hasValidTokens(dest.config.filename_pattern)) {
        errors.filename_pattern = "Pattern must include at least one token like {timestamp}";
      }

      // Extension validation (optional but if provided, should be valid)
      if (dest.config.extension) {
        const validExtensions = ['txt', 'md', 'json', 'log'];
        if (!validExtensions.includes(dest.config.extension)) {
          errors.extension = `Invalid extension. Use one of: ${validExtensions.join(', ')}`;
        }
      }
      break;

    case "telegram":
      // Credential validation
      if (!dest.config.credential_id?.trim()) {
        errors.credential_id = "Please select or create Telegram credentials";
      }

      // Chat ID validation
      if (!dest.config.chat_id?.trim()) {
        errors.chat_id = "Chat ID is required";
      } else if (!isValidChatId(dest.config.chat_id)) {
        errors.chat_id = "Invalid chat ID format. Use numeric ID or @channelname";
      }
      break;

    case "active_window":
      // Paste method validation
      const validPasteMethods = ['cmd_v', 'ctrl_v', 'ctrl_shift_v'];
      if (dest.config.paste_method && !validPasteMethods.includes(dest.config.paste_method)) {
        errors.paste_method = `Invalid paste method. Use one of: ${validPasteMethods.join(', ')}`;
      }
      break;

    default:
      errors._general = `Unknown destination type: ${dest.config.type}`;
  }

  return errors;
};

// Map backend validation errors to field-specific errors
export const mapBackendErrors = (error: any): ValidationErrors => {
  const errors: ValidationErrors = {};

  if (typeof error === "string") {
    // Parse known backend validation messages
    if (error.includes("Path cannot be empty") || error.includes("InvalidPath")) {
      errors.path = "Path is required";
    } else if (error.includes("Chat ID cannot be empty") || error.includes("InvalidChatId")) {
      errors.chat_id = "Chat ID is required";
    } else if (error.includes("Credential ID cannot be empty") || error.includes("InvalidCredentialId")) {
      errors.credential_id = "Telegram credentials are required";
    } else if (error.includes("Name cannot be empty") || error.includes("EmptyName")) {
      errors.name = "Name is required";
    } else if (error.includes("already exists")) {
      errors.name = "A destination with this name already exists";
    } else {
      // Generic error
      errors._general = error;
    }
  } else if (error?.message) {
    return mapBackendErrors(error.message);
  } else {
    errors._general = "An unexpected error occurred";
  }

  return errors;
};

// Check if form is valid (no errors)
export const isFormValid = (dest: Destination): boolean => {
  const errors = validateDestination(dest);
  return Object.keys(errors).length === 0;
};

// Helper to get default template for destination type
export const getDefaultTemplate = (type: string): string => {
  switch (type) {
    case "file_system":
      return `---
created: {timestamp}
model: {model_name}
workflow: {workflow_name}
duration: {duration}
---

{transcription_text}`;

    case "telegram":
      return `[{timestamp}] {workflow_name}

{transcription_text}

---
Model: {model_name} | Duration: {duration}`;

    case "active_window":
      return "{transcription_text}";

    default:
      return "{transcription_text}";
  }
};

// Export type label helper
export const typeLabel = (type: string): string => {
  switch (type) {
    case "active_window":
      return "Active Window";
    case "file_system":
      return "File System";
    case "telegram":
      return "Telegram";
    default:
      return "Unknown";
  }
};