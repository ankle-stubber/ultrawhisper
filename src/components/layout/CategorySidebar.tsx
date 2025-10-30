import React from "react";
import {
  Workflow,
  Send,
  Cpu,
  History,
  ScrollText,
} from "lucide-react";
import { Category } from "../../lib/types";
import UltraWhisperTextLogo from "../icons/UltraWhisperTextLogo";

interface CategoryConfig {
  id: Category;
  label: string;
  icon: React.ComponentType<any>;
}

const CATEGORIES: CategoryConfig[] = [
  { id: "workflows", label: "Workflows", icon: Workflow },
  { id: "destinations", label: "Destinations", icon: Send },
  { id: "models", label: "Models", icon: Cpu },
  { id: "history", label: "History", icon: History },
  { id: "logs", label: "Logs", icon: ScrollText },
];

interface CategorySidebarProps {
  activeCategory: Category;
  onCategoryChange: (category: Category) => void;
}

export const CategorySidebar: React.FC<CategorySidebarProps> = ({
  activeCategory,
  onCategoryChange,
}) => {
  return (
    <div className="flex flex-col w-40 h-full border-r border-mid-gray/20 items-center px-2">
      <UltraWhisperTextLogo width={120} className="m-4" />
      <div className="flex flex-col w-full items-center gap-1 pt-2 border-t border-mid-gray/20">
        {CATEGORIES.map((category) => {
          const Icon = category.icon;
          const isActive = activeCategory === category.id;

          return (
            <div
              key={category.id}
              className={`flex gap-2 items-center p-2 w-full rounded-lg cursor-pointer transition-colors ${
                isActive
                  ? "bg-logo-primary/80"
                  : "hover:bg-mid-gray/20 hover:opacity-100 opacity-85"
              }`}
              onClick={() => onCategoryChange(category.id)}
            >
              <Icon size={20} />
              <p className="text-sm font-medium">{category.label}</p>
            </div>
          );
        })}
      </div>
    </div>
  );
};
