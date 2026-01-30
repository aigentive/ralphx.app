/**
 * useIdeation hooks - TanStack Query wrappers for ideation session management
 *
 * Provides hooks for fetching, creating, archiving, and deleting ideation sessions
 * with automatic caching, refetching, and error handling.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ideationApi, type SessionWithDataResponse, type IdeationSessionResponse } from "@/api/ideation";

/**
 * Query key factory for ideation
 */
export const ideationKeys = {
  all: ["ideation"] as const,
  sessions: () => [...ideationKeys.all, "sessions"] as const,
  sessionList: (projectId: string) => [...ideationKeys.sessions(), "list", projectId] as const,
  sessionDetails: () => [...ideationKeys.sessions(), "detail"] as const,
  sessionDetail: (sessionId: string) => [...ideationKeys.sessionDetails(), sessionId] as const,
  sessionWithData: (sessionId: string) => [...ideationKeys.sessionDetail(sessionId), "with-data"] as const,
};

/**
 * Hook to fetch an ideation session with its proposals and messages
 *
 * @param sessionId - The session ID to fetch
 * @returns TanStack Query result with session data, proposals, and messages
 *
 * @remarks
 * This hook explicitly disables placeholder data to prevent a "flash" bug when
 * switching between sessions. The global queryClient config uses the previous
 * query data as placeholder, but for session switching we need clean transitions.
 * See resolvedSession in App.tsx for the full solution.
 *
 * @example
 * ```tsx
 * const { data, isLoading, error } = useIdeationSession("session-123");
 *
 * if (isLoading) return <Loading />;
 * if (error) return <Error message={error.message} />;
 * if (!data) return <NotFound />;
 *
 * return (
 *   <IdeationView
 *     session={data.session}
 *     proposals={data.proposals}
 *     messages={data.messages}
 *   />
 * );
 * ```
 */
export function useIdeationSession(sessionId: string) {
  return useQuery<SessionWithDataResponse | null, Error>({
    queryKey: ideationKeys.sessionWithData(sessionId),
    queryFn: () => ideationApi.sessions.getWithData(sessionId),
    enabled: Boolean(sessionId),
    /**
     * Explicitly return null instead of using global placeholderData.
     *
     * The global config preserves previous query data as placeholder, which causes
     * a flash when switching sessions: old session data briefly appears before
     * the new session loads. By returning null, we force clean transitions where
     * the consumer falls back to the store's activeSession.
     */
    placeholderData: () => null,
  });
}

/**
 * Hook to fetch all ideation sessions for a project
 *
 * @param projectId - The project ID to fetch sessions for
 * @returns TanStack Query result with sessions array
 *
 * @example
 * ```tsx
 * const { data: sessions, isLoading, error } = useIdeationSessions("project-123");
 *
 * if (isLoading) return <Loading />;
 * if (error) return <Error message={error.message} />;
 * return <SessionList sessions={sessions ?? []} />;
 * ```
 */
export function useIdeationSessions(projectId: string) {
  return useQuery<IdeationSessionResponse[], Error>({
    queryKey: ideationKeys.sessionList(projectId),
    queryFn: () => ideationApi.sessions.list(projectId),
    enabled: Boolean(projectId),
  });
}

/**
 * Input for creating a new ideation session
 */
interface CreateSessionInput {
  projectId: string;
  title?: string;
  seedTaskId?: string;
}

/**
 * Hook to create a new ideation session
 *
 * @returns Mutation object for creating sessions
 *
 * @example
 * ```tsx
 * const createSession = useCreateIdeationSession();
 *
 * const handleCreate = async () => {
 *   const session = await createSession.mutateAsync({
 *     projectId: "project-123",
 *     title: "New Feature Ideas",
 *   });
 *   navigate(`/ideation/${session.id}`);
 * };
 * ```
 */
export function useCreateIdeationSession() {
  const queryClient = useQueryClient();

  return useMutation<IdeationSessionResponse, Error, CreateSessionInput>({
    mutationFn: ({ projectId, title, seedTaskId }) => ideationApi.sessions.create(projectId, title, seedTaskId),
    onSuccess: (newSession) => {
      // Invalidate session list for the project to trigger refetch
      queryClient.invalidateQueries({
        queryKey: ideationKeys.sessionList(newSession.projectId),
      });
    },
  });
}

/**
 * Hook to archive an ideation session
 *
 * @returns Mutation object for archiving sessions
 *
 * @example
 * ```tsx
 * const archiveSession = useArchiveIdeationSession();
 *
 * const handleArchive = async (sessionId: string) => {
 *   await archiveSession.mutateAsync(sessionId);
 *   toast.success("Session archived");
 * };
 * ```
 */
export function useArchiveIdeationSession() {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: (sessionId) => ideationApi.sessions.archive(sessionId),
    onSuccess: (_data, sessionId) => {
      // Invalidate both the session detail and session lists
      queryClient.invalidateQueries({
        queryKey: ideationKeys.sessionDetail(sessionId),
      });
      queryClient.invalidateQueries({
        queryKey: ideationKeys.sessions(),
      });
    },
  });
}

/**
 * Hook to delete an ideation session
 *
 * @returns Mutation object for deleting sessions
 *
 * @example
 * ```tsx
 * const deleteSession = useDeleteIdeationSession();
 *
 * const handleDelete = async (sessionId: string) => {
 *   await deleteSession.mutateAsync(sessionId);
 *   navigate("/ideation");
 * };
 * ```
 */
export function useDeleteIdeationSession() {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: (sessionId) => ideationApi.sessions.delete(sessionId),
    onSuccess: (_data, sessionId) => {
      // Remove from cache and invalidate session lists
      queryClient.removeQueries({
        queryKey: ideationKeys.sessionDetail(sessionId),
      });
      queryClient.invalidateQueries({
        queryKey: ideationKeys.sessions(),
      });
    },
  });
}
