import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ecosystemApi } from "@/lib/api/ecosystem";

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

export function useCreateEcosystem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      name,
      description,
      frameworks,
    }: { name: string; description: string; frameworks?: string[] }) =>
      ecosystemApi.create(name, description, frameworks ?? []),
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

export function useInstallFramework() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ ecoId, frameworkId }: { ecoId: string; frameworkId: string }) =>
      ecosystemApi.installFramework(ecoId, frameworkId),
    onSuccess: (_, { ecoId }) => {
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "frameworks", ecoId] });
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "all"] });
    },
  });
}

export function useUninstallFramework() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ ecoId, frameworkId }: { ecoId: string; frameworkId: string }) =>
      ecosystemApi.uninstallFramework(ecoId, frameworkId),
    onSuccess: (_, { ecoId }) => {
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "frameworks", ecoId] });
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "all"] });
    },
  });
}

export function useUpdateFramework() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ ecoId, frameworkId }: { ecoId: string; frameworkId: string }) =>
      ecosystemApi.updateFramework(ecoId, frameworkId),
    onSuccess: (_, { ecoId }) => {
      queryClient.invalidateQueries({ queryKey: ["ecosystems", "frameworks", ecoId] });
    },
  });
}