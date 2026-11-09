import V3App, { type PlanCardExtension, type TechSettingsExtension } from '../../v3/src/App';
import { PlanPresetCard, PresetSettings } from './PresetSettings';

// `설정 > 앱 기술·구조`: PresetSettings keys its project panel by project ID, so a switch starts from a clean state.
const techSettings: TechSettingsExtension = context => <PresetSettings {...context} />;
const planCard: PlanCardExtension = context => <PlanPresetCard key={`${context.project?.id}:${context.goal.id}`} {...context} />;

export default function App() { return <V3App version="v4" techSettings={techSettings} planCard={planCard} />; }
