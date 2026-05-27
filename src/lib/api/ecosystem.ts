import { invoke } from "@tauri-apps/api/core";

export interface Ecosystem {
  id: string;
  name: string;
  description: string;
  isCurrent: boolean;
  createdAt: number;
}

export const ecosystemApi = {
  async list(): Promise<Ecosystem[]> {
    return await invoke("list_ecosystems");
  },

  async getCurrent(): Promise<Ecosystem | null> {
    return await invoke("get_current_ecosystem");
  },

  async create(name: string, description: string): Promise<Ecosystem> {
    return await invoke("create_ecosystem", { name, description });
  },

  async switch(id: string): Promise<void> {
    return await invoke("switch_ecosystem", { id });
  },

  async delete(id: string): Promise<void> {
    return await invoke("delete_ecosystem", { id });
  },
};