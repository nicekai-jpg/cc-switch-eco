import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ecosystemApi, type Ecosystem } from "@/lib/api/ecosystem";

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
    mutationFn: ({ name, description }: { name: string; description: string }) =>
      ecosystemApi.create(name, description),
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