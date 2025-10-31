import React from "react";
import { useThemeStore, THEME_PRESETS, ThemeName } from "../../stores/themeStore";
import { ConfigCard, ConfigField, PrimaryButton } from "../shared";
import { Palette } from "lucide-react";

export const AppearanceSettings: React.FC = () => {
  const { currentTheme, customColors, setTheme, updateCustomColor, resetCustomColors } = useThemeStore();

  const handleThemeSelect = (theme: ThemeName) => {
    setTheme(theme);
  };

  const handleColorChange = (key: keyof typeof customColors, value: string) => {
    updateCustomColor(key, value);
  };

  return (
    <div className="space-y-6">
      <ConfigCard title="Theme">
        <div>
          <label className="block text-sm font-medium uw-text-primary mb-3">
            Select Theme
          </label>
          <div className="grid grid-cols-2 gap-4">
            {(Object.keys(THEME_PRESETS) as ThemeName[]).map((themeKey) => {
              const theme = THEME_PRESETS[themeKey];
              const isActive = currentTheme === themeKey;

              return (
                <button
                  key={themeKey}
                  onClick={() => handleThemeSelect(themeKey)}
                  className={`
                    p-4 rounded-lg border-2 transition-all duration-200
                    ${isActive
                      ? "uw-border-primary uw-bg-primary-dim"
                      : "border-gray-700 hover:border-gray-600 uw-bg-elevated"
                    }
                  `}
                >
                  <div className="flex items-center gap-3">
                    <div
                      className="w-8 h-8 rounded-full border-2 border-gray-800"
                      style={{ backgroundColor: theme.primary }}
                    />
                    <div className="text-left">
                      <div className={`font-semibold text-sm ${isActive ? "uw-text-accent" : "uw-text-primary"}`}>
                        {theme.name}
                      </div>
                      <div className="text-xs uw-text-secondary">
                        {theme.description}
                      </div>
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      </ConfigCard>

      {currentTheme === "custom" && (
        <ConfigCard title="Custom Colors">
          <div className="space-y-4">
            <ConfigField
              label="Primary Accent"
              hint="Main theme color for buttons and highlights"
            >
              <div className="flex items-center gap-3">
                <input
                  type="color"
                  value={customColors.primary}
                  onChange={(e) => handleColorChange("primary", e.target.value)}
                  className="h-10 w-20 rounded border uw-border-default cursor-pointer"
                />
                <input
                  type="text"
                  value={customColors.primary}
                  onChange={(e) => handleColorChange("primary", e.target.value)}
                  className="flex-1 px-3 py-2 uw-bg-surface border uw-border-default rounded uw-mono text-sm uw-text-primary focus:outline-none focus:uw-border-primary"
                  placeholder="#1ed7d7"
                />
              </div>
            </ConfigField>

            <ConfigField
              label="Surface Background"
              hint="Main background color"
            >
              <div className="flex items-center gap-3">
                <input
                  type="color"
                  value={customColors.surface}
                  onChange={(e) => handleColorChange("surface", e.target.value)}
                  className="h-10 w-20 rounded border uw-border-default cursor-pointer"
                />
                <input
                  type="text"
                  value={customColors.surface}
                  onChange={(e) => handleColorChange("surface", e.target.value)}
                  className="flex-1 px-3 py-2 uw-bg-surface border uw-border-default rounded uw-mono text-sm uw-text-primary focus:outline-none focus:uw-border-primary"
                  placeholder="#0a0e1a"
                />
              </div>
            </ConfigField>

            <ConfigField
              label="Text Color"
              hint="Primary text color"
            >
              <div className="flex items-center gap-3">
                <input
                  type="color"
                  value={customColors.text}
                  onChange={(e) => handleColorChange("text", e.target.value)}
                  className="h-10 w-20 rounded border uw-border-default cursor-pointer"
                />
                <input
                  type="text"
                  value={customColors.text}
                  onChange={(e) => handleColorChange("text", e.target.value)}
                  className="flex-1 px-3 py-2 uw-bg-surface border uw-border-default rounded uw-mono text-sm uw-text-primary focus:outline-none focus:uw-border-primary"
                  placeholder="#e1e8f0"
                />
              </div>
            </ConfigField>

            <ConfigField
              label="Border Color"
              hint="Border and divider color"
            >
              <div className="flex items-center gap-3">
                <input
                  type="color"
                  value={customColors.border}
                  onChange={(e) => handleColorChange("border", e.target.value)}
                  className="h-10 w-20 rounded border uw-border-default cursor-pointer"
                />
                <input
                  type="text"
                  value={customColors.border}
                  onChange={(e) => handleColorChange("border", e.target.value)}
                  className="flex-1 px-3 py-2 uw-bg-surface border uw-border-default rounded uw-mono text-sm uw-text-primary focus:outline-none focus:uw-border-primary"
                  placeholder="#1a2332"
                />
              </div>
            </ConfigField>

            <div className="pt-2">
              <PrimaryButton
                onClick={resetCustomColors}
                variant="secondary"
                size="sm"
              >
                <Palette className="w-4 h-4 mr-2" />
                Reset to Defaults
              </PrimaryButton>
            </div>
          </div>
        </ConfigCard>
      )}

      <ConfigCard>
        <div className="flex items-start gap-3 p-4 uw-bg-card rounded-lg border uw-border-subtle">
          <div className="flex-shrink-0 mt-0.5">
            <div className="w-8 h-8 rounded-full uw-bg-primary-dim flex items-center justify-center">
              <Palette className="w-4 h-4 uw-text-accent" />
            </div>
          </div>
          <div className="flex-1">
            <h4 className="font-semibold uw-text-primary mb-1">Theme Preview</h4>
            <p className="text-sm uw-text-secondary">
              Changes apply instantly. Your theme preference is saved automatically and will persist across app restarts.
            </p>
          </div>
        </div>
      </ConfigCard>
    </div>
  );
};
