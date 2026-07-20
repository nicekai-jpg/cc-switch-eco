import { invoke } from "@tauri-apps/api/core";

export interface Ecosystem {
  id: string;
  name: string;
  description: string;
  isCurrent: boolean;
  createdAt: number;
}

export interface FrameworkRegistry {
  id: string;
  name: string;
  description: string;
  repoUrl: string;
  repoBranch: string;
  installMethod: string;
  filePrefix: string;
  providedDirs: string[];
  isolatedFiles: string[];
}

export interface FrameworkDetail {
  installedAt: number;
  commitHash: string;
}

export interface EcosystemStatus {
  ecoId: string;
  frameworks: string[];
  frameworkDetails: Record<string, FrameworkDetail>;
  mergeConflicts: Record<string, string[]>;
  installErrors: string[];
}

export interface CreateEcosystemResult {
  ecosystem: Ecosystem;
  installErrors: string[];
}

export const ecosystemApi = {
  async list(): Promise<Ecosystem[]> {
    return await invoke("list_ecosystems");
  },

  async getCurrent(): Promise<Ecosystem | null> {
    return await invoke("get_current_ecosystem");
  },

  async create(
    name: string,
    description: string,
    frameworks: string[] = [],
  ): Promise<CreateEcosystemResult> {
    return await invoke("create_ecosystem", { name, description, frameworks });
  },

  async switch(id: string): Promise<void> {
    return await invoke("switch_ecosystem", { id });
  },

  async delete(id: string): Promise<void> {
    return await invoke("delete_ecosystem", { id });
  },

  async listFrameworks(): Promise<FrameworkRegistry[]> {
    return await invoke("list_frameworks");
  },

  async installFramework(ecoId: string, frameworkId: string): Promise<void> {
    return await invoke("install_framework_to_ecosystem", {
      ecoId,
      frameworkId,
    });
  },

  async uninstallFramework(ecoId: string, frameworkId: string): Promise<void> {
    return await invoke("uninstall_framework_from_ecosystem", {
      ecoId,
      frameworkId,
    });
  },

  async updateFramework(ecoId: string, frameworkId: string): Promise<void> {
    return await invoke("update_framework_in_ecosystem", {
      ecoId,
      frameworkId,
    });
  },

  async getEcosystemFrameworks(ecoId: string): Promise<string[]> {
    return await invoke("get_ecosystem_frameworks", { ecoId });
  },

  async getEcosystemStatus(ecoId: string): Promise<EcosystemStatus> {
    return await invoke("get_ecosystem_status", { ecoId });
  },

  async saveUserPreferences(ecoId: string, fileName: string): Promise<void> {
    return await invoke("save_user_preferences", { ecoId, fileName });
  },

  async removeUserPreference(
    ecoId: string,
    fileName: string,
    keyPath: string,
  ): Promise<void> {
    return await invoke("remove_user_preference", { ecoId, fileName, keyPath });
  },
};
