/**
 * Explicit icon map.
 *
 * `import * as Lucide` pulls all ~5,987 exports into the client bundle. We use
 * ~30, so they are named individually here and tree-shaken properly. An unknown
 * name falls back to `Circle` rather than crashing.
 */
import {
  Activity, Atom, Binary, BookOpen, Boxes, Brain, BrainCircuit, Check, ChevronDown,
  ChevronRight, ChevronUp, Circle, ClipboardCheck, Contrast, Cpu, Database, FileCode2,
  FileText, GitCompare, Layers, LayoutDashboard, Menu, MonitorSmartphone, Moon, Play,
  Projector, Radar, Search, SearchX, Settings, Sparkles, Sun, Waypoints, X,
  ArrowRight, ArrowUpRight, AlertTriangle, Compass, GraduationCap, Wrench, FlaskConical, ScrollText, Pause, StepForward, Square,
  type LucideProps,
} from "lucide-react";
import type { ComponentType } from "react";

export const ICONS: Record<string, ComponentType<LucideProps>> = {
  Activity, Atom, Binary, BookOpen, Boxes, Brain, BrainCircuit, Check, ChevronDown,
  ChevronRight, ChevronUp, Circle, ClipboardCheck, Contrast, Cpu, Database, FileCode2,
  FileText, GitCompare, Layers, LayoutDashboard, Menu, MonitorSmartphone, Moon, Play,
  Projector, Radar, Search, SearchX, Settings, Sparkles, Sun, Waypoints, X,
  ArrowRight, ArrowUpRight, AlertTriangle, Compass, GraduationCap, Wrench, FlaskConical, ScrollText, Pause, StepForward, Square,
};

export const FALLBACK_ICON = Circle;
export type { LucideProps };
