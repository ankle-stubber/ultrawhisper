import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SettingContainer } from "../ui/SettingContainer";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

interface TelegramConfig {
  type: "telegram";
  credential_id: string;
  chat_id: string;
  include_audio: boolean;
}

interface TelegramSetupProps {
  config: TelegramConfig;
  onChange?: (field: string, value: any) => void;  // Creation mode
  onSave?: (config: TelegramConfig) => Promise<void>;  // Edit mode
  errors?: Record<string, string>;
  requireCredentialSelection?: boolean;
}

interface TelegramTestResult {
  success: boolean;
  message: string;
  bot_username?: string;
}

export const TelegramSetup: React.FC<TelegramSetupProps> = ({
  config,
  onChange,
  onSave,
  errors = {},
  requireCredentialSelection = false,
}) => {
  // Form state
  const [credentialId, setCredentialId] = useState(config.credential_id || "");
  const [chatId, setChatId] = useState(config.chat_id || "");
  const [includeAudio, setIncludeAudio] = useState(config.include_audio || false);

  // UI state
  const [botToken, setBotToken] = useState("");
  const [testResult, setTestResult] = useState<TelegramTestResult | null>(null);
  const [isTesting, setIsTesting] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [hasCredentials, setHasCredentials] = useState(false);

  // Original values for dirty tracking
  const [originalConfig, setOriginalConfig] = useState({
    credential_id: config.credential_id,
    chat_id: config.chat_id,
    include_audio: config.include_audio,
  });

  // Update form when config prop changes
  useEffect(() => {
    setCredentialId(config.credential_id || "");
    setChatId(config.chat_id || "");
    setIncludeAudio(config.include_audio || false);
    setOriginalConfig({
      credential_id: config.credential_id,
      chat_id: config.chat_id,
      include_audio: config.include_audio,
    });
  }, [config]);

  // Check if credentials exist
  useEffect(() => {
    if (credentialId) {
      checkExistingCredentials();
    }
  }, [credentialId]);

  const checkExistingCredentials = async () => {
    if (!credentialId) return;

    try {
      const exists = await invoke<boolean>("telegram_credentials_exist", {
        credentialId,
      });
      setHasCredentials(exists);
    } catch (error) {
      console.error("Failed to check credentials:", error);
    }
  };

  // Dirty detection
  const isDirty =
    credentialId !== originalConfig.credential_id ||
    chatId !== originalConfig.chat_id ||
    includeAudio !== originalConfig.include_audio;

  // Handle field changes
  const handleFieldChange = (field: string, value: any) => {
    // Update local state
    if (field === "credential_id") {
      setCredentialId(value);
    } else if (field === "chat_id") {
      setChatId(value);
    } else if (field === "include_audio") {
      setIncludeAudio(value);
    }

    // Notify parent (for creation mode)
    if (onChange) {
      onChange(field, value);
    }

    // Persistence handled via explicit Save button
  };

  const handleTest = async () => {
    if (!botToken.trim() && !hasCredentials) {
      setTestResult({
        success: false,
        message: "Please enter a bot token or save credentials first",
      });
      return;
    }

    if (!chatId.trim()) {
      setTestResult({
        success: false,
        message: "Please enter a chat ID",
      });
      return;
    }

    setIsTesting(true);
    setTestResult(null);

    try {
      // If we have saved credentials, test with those
      if (hasCredentials && !botToken.trim()) {
        const result = await invoke<TelegramTestResult>("test_telegram_destination", {
          credentialId: credentialId.trim(),
          chatId: chatId.trim(),
        });
        setTestResult(result);
      } else {
        // Test with provided bot token
        const result = await invoke<TelegramTestResult>("test_telegram_connection", {
          botToken: botToken.trim(),
          chatId: chatId.trim(),
        });
        setTestResult(result);
      }
    } catch (error) {
      setTestResult({
        success: false,
        message: `Test failed: ${error}`,
      });
    } finally {
      setIsTesting(false);
    }
  };

  const handleSaveCredentials = async () => {
    if (!botToken.trim()) {
      setTestResult({
        success: false,
        message: "Please enter a bot token",
      });
      return;
    }

    setIsSaving(true);
    setTestResult(null);

    try {
      // Generate credential ID if not set
      const finalCredentialId = credentialId || `telegram_${crypto.randomUUID().slice(0, 8)}`;

      // Store credentials in keychain
      await invoke("store_telegram_credentials", {
        credentialId: finalCredentialId,
        botToken: botToken.trim(),
      });

      setTestResult({
        success: true,
        message: "Credentials saved successfully to OS keychain",
      });

      setHasCredentials(true);
      setBotToken(""); // Clear the input for security

      // Update credential ID if it was generated
      if (!credentialId) {
        handleFieldChange("credential_id", finalCredentialId);
      }
    } catch (error) {
      setTestResult({
        success: false,
        message: `Failed to save credentials: ${error}`,
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleSave = async () => {
    if (!onSave) return;

    setIsSaving(true);
    try {
      await onSave({
        type: "telegram",
        credential_id: credentialId,
        chat_id: chatId,
        include_audio: includeAudio,
      });

      // Reset baseline after successful save
      setOriginalConfig({
        credential_id: credentialId,
        chat_id: chatId,
        include_audio: includeAudio,
      });
    } catch (error) {
      console.error("Save error:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const handleRevert = () => {
    setCredentialId(originalConfig.credential_id);
    setChatId(originalConfig.chat_id);
    setIncludeAudio(originalConfig.include_audio);
  };

  const handleDelete = async () => {
    if (!confirm("Are you sure you want to delete the stored credentials?")) {
      return;
    }

    try {
      await invoke("delete_telegram_credentials", { credentialId });
      setTestResult({
        success: true,
        message: "Credentials deleted successfully",
      });
      setHasCredentials(false);
      setBotToken("");
      handleFieldChange("credential_id", "");
    } catch (error) {
      setTestResult({
        success: false,
        message: `Failed to delete credentials: ${error}`,
      });
    }
  };

  return (
    <div className="space-y-4">
      <SettingContainer
        title="Telegram Configuration"
        description="Configure your Telegram bot to send transcriptions"
        descriptionMode="inline"
        grouped={false}
      >
        <div className="space-y-4">
          {/* Credential Status/Setup */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Bot Credentials
              <span className="text-red-500 ml-1" aria-label="required">*</span>
            </label>

            {hasCredentials ? (
              <div className="flex items-center gap-2 mb-2">
                <span className="text-green-500">✓ Connected</span>
                <Button
                  onClick={handleDelete}
                  variant="danger"
                  size="sm"
                  disabled={isTesting || isSaving}
                >
                  Delete
                </Button>
              </div>
            ) : (
              <div>
                <Input
                  type="password"
                  value={botToken}
                  onChange={(e) => setBotToken(e.target.value)}
                  placeholder="Enter bot token from @BotFather"
                  className={`w-full mb-2 ${errors.credential_id ? 'border-red-500' : ''}`}
                  disabled={isTesting || isSaving}
                  aria-invalid={!!errors.credential_id}
                  aria-describedby={errors.credential_id ? "credential-error" : "credential-help"}
                />
                <Button
                  onClick={handleSaveCredentials}
                  disabled={!botToken.trim() || isSaving}
                  variant="primary"
                  size="sm"
                >
                  {isSaving ? "Saving..." : "Save Credentials"}
                </Button>
                {errors.credential_id && (
                  <p id="credential-error" className="text-red-500 text-sm mt-1">
                    {errors.credential_id}
                  </p>
                )}
              </div>
            )}

            <p id="credential-help" className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Create a bot via @BotFather on Telegram to get your bot token
            </p>
          </div>

          {/* Chat ID Input */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Chat ID
              <span className="text-red-500 ml-1" aria-label="required">*</span>
            </label>
            <Input
              type="text"
              value={chatId}
              onChange={(e) => handleFieldChange("chat_id", e.target.value)}
              placeholder="Enter chat ID (e.g., -123456789 or @channel)"
              className={`w-full ${errors.chat_id ? 'border-red-500' : ''}`}
              disabled={isTesting || isSaving}
              aria-invalid={!!errors.chat_id}
              aria-describedby="chat-id-help chat-id-error"
            />
            {errors.chat_id && (
              <p id="chat-id-error" className="text-red-500 text-sm mt-1">
                {errors.chat_id}
              </p>
            )}
            <p id="chat-id-help" className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Use @userinfobot to get your chat ID, or use @channelname for public channels
            </p>
          </div>

          {/* Include Audio Checkbox */}
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="include-audio"
              checked={includeAudio}
              onChange={(e) => handleFieldChange("include_audio", e.target.checked)}
              disabled={isSaving}
              className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
            />
            <label htmlFor="include-audio" className="text-sm text-gray-700 dark:text-gray-300">
              Include audio file with transcription
            </label>
          </div>

          {/* Test Connection Button */}
          {(hasCredentials || botToken) && chatId && (
            <div className="pt-2">
              <Button
                onClick={handleTest}
                disabled={isTesting || isSaving || !chatId.trim()}
                variant="secondary"
              >
                {isTesting ? "Testing..." : "Test Connection"}
              </Button>
            </div>
          )}

          {/* Save/Revert buttons for edit mode */}
          {onSave && isDirty && (
            <div className="flex gap-2 pt-2">
              <Button
                onClick={handleRevert}
                disabled={isSaving}
                variant="secondary"
              >
                Revert
              </Button>
              <Button
                onClick={handleSave}
                disabled={isSaving || !credentialId || !chatId}
                variant="primary"
              >
                {isSaving ? "Saving..." : "Save"}
              </Button>
            </div>
          )}

          {/* Test Result */}
          {testResult && (
            <div
              className={`p-3 rounded-md ${
                testResult.success
                  ? "bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-200"
                  : "bg-red-50 dark:bg-red-900/20 text-red-800 dark:text-red-200"
              }`}
            >
              <p className="text-sm font-medium">
                {testResult.success ? "✅ Success" : "❌ Error"}
              </p>
              <p className="text-sm mt-1">{testResult.message}</p>
              {testResult.bot_username && (
                <p className="text-xs mt-1 opacity-80">
                  Bot: @{testResult.bot_username}
                </p>
              )}
            </div>
          )}

          {/* Credential Status Info */}
          {hasCredentials && !testResult && !requireCredentialSelection && (
            <div className="p-3 rounded-md bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-200">
              <p className="text-sm">
                ℹ️ Credentials are stored securely in your system keychain
              </p>
            </div>
          )}
        </div>
      </SettingContainer>
    </div>
  );
};
