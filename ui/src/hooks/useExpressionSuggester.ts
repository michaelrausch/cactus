import { useMemo } from "react";
import { expressionSuggester, type Suggester } from "@/domain/suggestions";
import { useResponseStore } from "@/stores/responseStore";

/** Completes `res.…` expressions, with body keys and headers from the response on show. */
export function useExpressionSuggester(): Suggester {
  const response = useResponseStore((s) => s.response);
  return useMemo(() => expressionSuggester(response), [response]);
}
