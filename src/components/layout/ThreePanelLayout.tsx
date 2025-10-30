import React from "react";
import { Category } from "../../lib/types";
import { CategorySidebar } from "./CategorySidebar";

interface ThreePanelLayoutProps {
  activeCategory: Category;
  onCategoryChange: (category: Category) => void;
  selectedItemId: string | null;
  onItemSelect: (itemId: string | null) => void;
  itemsPanel: React.ReactNode;
  detailPanel: React.ReactNode;
}

export const ThreePanelLayout: React.FC<ThreePanelLayoutProps> = ({
  activeCategory,
  onCategoryChange,
  itemsPanel,
  detailPanel,
}) => {
  return (
    <div className="flex h-full overflow-hidden">
      {/* Left: Category Sidebar (160px) */}
      <CategorySidebar
        activeCategory={activeCategory}
        onCategoryChange={onCategoryChange}
      />

      {/* Middle: Items List (280px) */}
      <div className="w-[280px] border-r border-mid-gray/20 flex flex-col overflow-hidden">
        {itemsPanel}
      </div>

      {/* Right: Detail Panel (flex-1) */}
      <div className="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0">
        {detailPanel}
      </div>
    </div>
  );
};
