import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SettingContainer } from "../ui/SettingContainer";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

interface TelegramSetupProps {
  credentialId: string;
  initialChatId?: string;
  onSave?: (chatId: string) => void;
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

interface TelegramTestResult {
  success: boolean;
  message: string;
  bot_username?: string;
}

export const TelegramSetup: React.FC<TelegramSetupProps> = ({
  credentialId,
  initialChatId = "",
  onSave,
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const [botToken, setBotToken] = useState("");
  const [chatId, setChatId] = useState(initialChatId);
  const [testResult, setTestResult] = useState<TelegramTestResult | null>(null);
  const [isTesting, setIsTesting] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [hasCredentials, setHasCredentials] = useState(false);

  React.useEffect(() => {
    // Check if credentials already exist
    checkExistingCredentials();
  }, [credentialId]);

  const checkExistingCredentials = async () => {
    try {
      const exists = await invoke<boolean>("telegram_credentials_exist", {
        credentialId,
      });
      setHasCredentials(exists);
    } catch (error) {
      console.error("Failed to check credentials:", error);
    }
  };

  const handleTest = async () => {
    if (!botToken.trim() || !chatId.trim()) {
      setTestResult({
        success: false,
        message: "Please enter both bot token and chat ID",
      });
      return;
    }

    setIsTesting(true);
    setTestResult(null);

    try {
      const result = await invoke<TelegramTestResult>("test_telegram_connection", {
        botToken: botToken.trim(),
        chatId: chatId.trim(),
      });
      setTestResult(result);
    } catch (error) {
      setTestResult({
        success: false,
        message: `Test failed: ${error}`,
      });
    } finally {
      setIsTesting(false);
    }
  };

  const handleSave = async () => {
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
      // Store credentials in keychain
      await invoke("store_telegram_credentials", {
        credentialId,
        botToken: botToken.trim(),
      });

      setTestResult({
        success: true,
        message: "Credentials saved successfully to OS keychain",
      });

      setHasCredentials(true);
      setBotToken(""); // Clear the input for security

      // Notify parent component
      if (onSave) {
        onSave(chatId.trim());
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
    } catch (error) {
      setTestResult({
        success: false,
        message: `Failed to delete credentials: ${error}`,
      });
    }
  };

  return (
    <SettingContainer
      title="Telegram Configuration"
      description="Configure your Telegram bot to send transcriptions"
      descriptionMode={descriptionMode}
      grouped={grouped}
    >
      <div className="space-y-4">
        {/* Bot Token Input */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Bot Token
          </label>
          <Input
            type="password"
            value={botToken}
            onChange={(e) => setBotToken(e.target.value)}
            placeholder={hasCredentials ? "Token stored in keychain" : "Enter bot token"}
            className="w-full"
            disabled={isTesting || isSaving}
          />
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            Create a bot via @BotFather on Telegram
          </p>
        </div>

        {/* Chat ID Input */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Chat ID
          </label>
          <Input
            type="text"
            value={chatId}
            onChange={(e) => setChatId(e.target.value)}
            placeholder="Enter chat ID"
            className="w-full"
            disabled={isTesting || isSaving}
          />
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            Use @userinfobot to get your chat ID
          </p>
        </div>

        {/* Action Buttons */}
        <div className="flex gap-2">
          <Button
            onClick={handleTest}
            disabled={isTesting || isSaving || !botToken.trim() || !chatId.trim()}
            variant="secondary"
          >
            {isTesting ? "Testing..." : "Test Connection"}
          </Button>

          <Button
            onClick={handleSave}
            disabled={isTesting || isSaving || !botToken.trim()}
            variant="primary"
          >
            {isSaving ? "Saving..." : "Save Credentials"}
          </Button>

          {hasCredentials && (
            <Button
              onClick={handleDelete}
              disabled={isTesting || isSaving}
              variant="danger"
            >
              Delete Credentials
            </Button>
          )}
        </div>

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

        {/* Credential Status */}
        {hasCredentials && !testResult && (
          <div className="p-3 rounded-md bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-200">
            <p className="text-sm">
              ℹ️ Credentials are stored securely in your system keychain
            </p>
          </div>
        )}
      </div>
    </SettingContainer>
  );
};
