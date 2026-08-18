import { graph } from "@/data";
import LLMCenter from "@/components/os/llm/LLMCenter";
import SystemPanel from "@/components/os/control/SystemPanel";

export default function LLMPage() {
  // Real knowledge-graph facts become the Ideator prompt digest (as llm.rs does).
  const facts = graph.facts.map((f) => ({
    subject: f.subject, relation: f.relation, object: f.object,
    weight: f.weight, support: f.support, confidence: f.confidence,
  }));
  return (
    <>
      <LLMCenter facts={facts} />
      <div className="mx-auto max-w-[1240px] px-5 pb-8 md:px-8">
        <SystemPanel />
      </div>
    </>
  );
}
