import { create } from "zustand";
import { Category } from "../lib/types";

interface NavigationStore {
  activeCategory: Category;
  setActiveCategory: (category: Category) => void;
}

export const useNavigationStore = create<NavigationStore>((set) => ({
  activeCategory: "workflows",
  setActiveCategory: (category: Category) => set({ activeCategory: category }),
}));
