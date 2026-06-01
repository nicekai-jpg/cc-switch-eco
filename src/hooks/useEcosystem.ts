import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ecosystemApi } from "@/lib/api/ecosystem";

// === 查询 Hooks ===

export function useAllEcosystems() {
  return useQuery({
    queryKey: ["ecosystems", "all"],
    queryFn: () => ecosystemApi.list(),
  });
}

export function useCurrentEcosystem() {
  return useQuery({
    queryKey: ["ecosystems", "current"],
    queryFn: () => ecosystemApi.getCurrent(),
  });
}

export function useAllFrameworks() {
  return useQuery({
    queryKey: ["ecosystems", "frameworks"],
    queryFn: () => ecosystemApi.listFrameworks(),
  });
}

export function useEcosystemFrameworks(ecoId: string | undefined) {
  return useQuery({
    queryKey: ["ecosystems", "frameworks", ecoId],
    queryFn: () => ecosystemApi.getEcosystemFrameworks(ecoId!),
    enabled: !!ecoId,
  });
}

// === 变更 Hooks ===

export function useCreateEcosystem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      name,
      description,
      frameworks,
    }: {
      name: string;
      description: string;
      frameworks?: string[];
    }) => ecosystemApi.create(name, description, frameworks ?? []),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "all"] });
    },
  });
}

export function useSwitchEcosystem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => ecosystemApi.switch(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "all"] });
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "current"] });
    },
  });
}

export function useDeleteEcosystem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => ecosystemApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "all"] });
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "current"] });
    },
  });
}

// === 框架管理 Hooks ===

export function useInstallFramework() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      ecoId,
      frameworkId,
    }: {
      ecoId: string;
      frameworkId: string;
    }) => ecosystemApi.installFramework(ecoId, frameworkId),
    onSuccess: (_, { ecoId }) => {
      queryClient.invalidateQueries({
        queryKey: ["ecosystems", "frameworks", ecoId],
      });
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "all"] });
    },
  });
}

export function useUninstallFramework() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      ecoId,
      frameworkId,
    }: {
      ecoId: string;
      frameworkId: string;
    }) => ecosystemApi.uninstallFramework(ecoId, frameworkId),
    onSuccess: (_, { ecoId }) => {
      queryClient.invalidateQueries({
        queryKey: ["ecosystems", "frameworks", ecoId],
      });
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "all"] });
    },
  });
}

export function useUpdateFramework() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      ecoId,
      frameworkId,
    }: {
      ecoId: string;
      frameworkId: string;
    }) => ecosystemApi.updateFramework(ecoId, frameworkId),
    onSuccess: (_, { ecoId }) => {
      queryClient.invalidateQueries({
        queryKey: ["ecosystems", "frameworks", ecoId],
      });
    },
  });
}

// === 用户偏好 Hooks ===

export function useSaveUserPreferences() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      ecoId,
      fileName,
    }: {
      ecoId: string;
      fileName: string;
    }) => ecosystemApi.saveUserPreferences(ecoId, fileName),
    onSuccess: (_, { ecoId }) => {
      queryClient.invalidateQueries({
        queryKey: ["ecosystems", "frameworks", ecoId],
      });
    },
  });
}

export function useRemoveUserPreference() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      ecoId,
      fileName,
      keyPath,
    }: {
      ecoId: string;
      fileName: string;
      keyPath: string;
    }) => ecosystemApi.removeUserPreference(ecoId, fileName, keyPath),
    onSuccess: (_, { ecoId }) => {
      queryClient.invalidateQueries({
        queryKey: ["ecosystems", "frameworks", ecoId],
      });
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "all"] });
    },
  });
}
