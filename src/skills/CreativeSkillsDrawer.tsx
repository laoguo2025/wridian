import type { CreativeSkill, CreativeSkillId } from "../creativeSkills";

export function CreativeSkillsDrawer({
  enabled,
  onClose,
  onToggle,
  skills,
}: {
  enabled: Record<CreativeSkillId, boolean>;
  onClose: () => void;
  onToggle: (id: CreativeSkillId) => void;
  skills: CreativeSkill[];
}) {
  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer creative-skills-drawer" role="dialog" aria-modal="true" aria-label="技能管理" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">技能管理</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </div>

        <div className="creative-skills-list">
          {skills.map((skill) => (
            <div className="creative-skill-row" key={skill.id}>
              <div className="creative-skill-main">
                <div className="creative-skill-title">{skill.title}</div>
                <div className="creative-skill-meta">{skill.status}</div>
              </div>
              <button
                type="button"
                className={enabled[skill.id] ? "skill-toggle active" : "skill-toggle"}
                aria-pressed={enabled[skill.id]}
                onClick={() => onToggle(skill.id)}
              >
                <span />
              </button>
            </div>
          ))}
        </div>
      </aside>
    </div>
  );
}
