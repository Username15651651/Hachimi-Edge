use crate::{
    core::{Hachimi, game::Region, tl_repo, utils::{mul_int, str_visual_len}},
    il2cpp::{ext::{Il2CppStringExt, StringExt}, hook::{UnityEngine_CoreModule::{Component, Object, UnityAction}, UnityEngine_UI::{EventSystem, Text}}, sql::{self, TextDataQuery}, symbols::{create_delegate, get_field_from_name, get_field_object_value, get_method_addr}, types::*}
};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use fnv::FnvHashMap;
use super::{ButtonCommon, DialogCommon, DialogManager, MasterDataUtil};

static SKILL_TEXT_CACHE: Lazy<Mutex<FnvHashMap<i32, (String, String)>>> = Lazy::new(|| Mutex::default());

// SkillListItem
static mut NAMETEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get__nameText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAMETEXT_FIELD })
}
static mut DESCTEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get__descText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { DESCTEXT_FIELD })
}

static mut _BGBUTTON_FIELD: *mut FieldInfo = 0 as _;
pub fn get__bgButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { _BGBUTTON_FIELD })
}

static mut INFO_FIELD: *mut FieldInfo = 0 as _;
pub fn get_info(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { INFO_FIELD })
}

static mut set_skill_name_text_addr: usize = 0;
impl_addr_wrapper_fn!(set_skill_name_text, set_skill_name_text_addr, (), this: *mut Il2CppObject);

// PartsSingleModeSkillListItem.Info
static mut get_IsDrawDesc_addr: usize = 0;
impl_addr_wrapper_fn!(get_IsDrawDesc, get_IsDrawDesc_addr, bool, this: *mut Il2CppObject);
static mut get_IsDrawNeedSkillPoint_addr: usize = 0;
impl_addr_wrapper_fn!(get_IsDrawNeedSkillPoint, get_IsDrawNeedSkillPoint_addr, bool, this: *mut Il2CppObject);
static mut get_Id_addr: usize = 0;
impl_addr_wrapper_fn!(get_Id, get_Id_addr, i32, this: *mut Il2CppObject);

fn UpdateItemCommon(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, orig_fn_cb: impl FnOnce()) {
    let skill_cfg = &Hachimi::instance().localized_data.load().config.skill_formatting;
    let mut txt_cfg = sql::SkillTextFormatting::default();

    let name = get__nameText(this);
    let desc = get__descText(this);

    // Name should always exist, but let's be sure.
    if !name.is_null() {
        let mut name_len = skill_cfg.name_length;
        let mut name_lines = 1;

        // Uma info, "short ver".
        if !get_IsDrawDesc(skill_info) {
            name_len = mul_int(name_len, skill_cfg.name_short_mult);
            name_lines = skill_cfg.name_short_lines;
        }
        // "Draw Skill Pt" is also true on the short ver, even though it doesn't show there.
        // So, apply only when desc shows.
        else if get_IsDrawNeedSkillPoint(skill_info) {
            name_len = mul_int(name_len, skill_cfg.name_sp_mult);
        }
        // todo: When lvl display!?
        // if get_IsDrawUniqSkillInfo(skill_info) || get_Level(skill_info) > 1 {
        //     name_len = mul_int(name_len, skill_cfg.name_lvl_mult);
        // }

        txt_cfg.name = Some(sql::TextFormatting {
            line_len: name_len,
            line_count: name_lines,
            font_size: Text::get_fontSize(name)
        });
    }

    if get_IsDrawDesc(skill_info) && !desc.is_null() {
        let desc_len = skill_cfg.desc_length;
        // todo: When conditions button!?
        // if get_IsDisplayUpgradeSkill(skill_info) {
        //     desc_len = mul_int(desc_len, skill_cfg.desc_btn_mult);
        // }

        txt_cfg.desc = Some(sql::TextFormatting {
            line_len: desc_len,
            line_count: 4,
            font_size: Text::get_fontSize(desc)
        });
    }

    TextDataQuery::with_skill_query(&txt_cfg, orig_fn_cb);

    if txt_cfg.is_localized {
        if !name.is_null() {
            Text::set_horizontalOverflow(name, 1);
            if txt_cfg.name.map(|opts| opts.line_count).unwrap_or(1) > 1 {
                Text::set_verticalOverflow(name, 1);
            }
        }
        if !desc.is_null() {
            Text::set_horizontalOverflow(desc, 1);
        }
    }
    
    // 在这里设置技能详情的点击事件，替代原来的SetupOnClickSkillButton hook
    if Hachimi::instance().config.load().skill_info_dialog {
        let skill_id = get_Id(skill_info);
        let button = get__bgButton(this);
        if !button.is_null() {
            let button_obj = Component::get_gameObject(button);
            Object::set_name(button_obj, format!("HachimiSkill_{}", skill_id).to_il2cpp_string());
            get_skill_text(skill_id);

            let delegate = create_delegate(unsafe { UnityAction::UNITYACTION_CLASS }, 0, || {
                let current_ev = EventSystem::get_current();
                let clicked_obj = EventSystem::get_currentSelectedGameObject(current_ev);
                let object_name = Object::get_name(clicked_obj);
                let name_str = unsafe { (*object_name).as_utf16str() }.to_string();

                if name_str.starts_with("HachimiSkill_") {
                    let id_str = &name_str["HachimiSkill_".len()..];
                    if let Ok(id) = id_str.parse::<i32>() {
                        if let Some(data) = SKILL_TEXT_CACHE.lock().unwrap().get(&id) {
                            let (name, desc) = data;
                            let typ = if str_visual_len(desc.as_str()) <= 250 {
                                DialogCommon::FormType::SMALL_ONE_BUTTON
                            } else if str_visual_len(desc.as_str()) <= 490 {
                                DialogCommon::FormType::MIDDLE_ONE_BUTTON
                            } else {
                                DialogCommon::FormType::BIG_ONE_BUTTON
                            };
                            DialogManager::single_button_message(name, &desc.replace("\\n", "\n"), typ);
                        }
                    }
                }
            });
            if let Some(delegate) = delegate {
                ButtonCommon::SetOnClick(button, delegate);
            }
        }
    }
}

type UpdateItemJpFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, adjuster_data: *mut Il2CppObject, resource_hash: i32);
extern "C" fn UpdateItemJp(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, adjuster_data: *mut Il2CppObject, resource_hash: i32) {
    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemJp, UpdateItemJpFn)(this, skill_info, is_plate_effect_enable, adjuster_data, resource_hash);
    });
}

type UpdateItemOtherFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool);
extern "C" fn UpdateItemOther(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool) {
    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemOther, UpdateItemOtherFn)(this, skill_info, is_plate_effect_enable);
    });
}

fn get_skill_text(skill_id: i32) -> (String, String) {
    // 先检查用户自定义的技能详情
    let details = tl_repo::SKILL_DETAILS.lock().unwrap();
    if let Some(custom_detail) = details.get(&(skill_id as u32)) {
        // 解析用户自定义的详情：第一行是名字，剩下的是描述
        let mut lines = custom_detail.lines();
        let name = lines.next().unwrap_or("").to_string();
        let desc = lines.collect::<Vec<_>>().join("\n");
        
        let mut cache = SKILL_TEXT_CACHE.lock().unwrap();
        cache.insert(skill_id, (name.clone(), desc.clone()));
        return (name, desc);
    }
    
    // 没有自定义详情，使用原来的逻辑
    let to_s = |opt_ptr: Option<*mut Il2CppString>| unsafe {
        opt_ptr.and_then(|p| p.as_ref()).map(|s| s.as_utf16str().to_string())
    };

    let current_name = to_s(TextDataQuery::get_skill_name(skill_id)).unwrap_or_else(|| to_s(Some(MasterDataUtil::GetSkillName(skill_id))).unwrap());
    let current_desc = to_s(TextDataQuery::get_skill_desc(skill_id)).unwrap_or_else(|| to_s(
        Some(Hachimi::instance().skill_info.load().get_desc(skill_id).to_il2cpp_string())
    ).unwrap());

    let mut cache = SKILL_TEXT_CACHE.lock().unwrap();

    if let Some((cached_name, cached_desc)) = cache.get(&skill_id) {
        if cached_name == &current_name && cached_desc == &current_desc {
            return (cached_name.clone(), cached_desc.clone());
        }
    }

    cache.insert(skill_id, (current_name.clone(), current_desc.clone()));
    (current_name, current_desc)
}



pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsSingleModeSkillListItem);
    find_nested_class_or_return!(PartsSingleModeSkillListItem, Info);

    if Hachimi::instance().game.region == Region::Japan {
        let UpdateItem_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 4);
        new_hook!(UpdateItem_addr, UpdateItemJp);
    }
    else {
        let UpdateItem_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 2);
        new_hook!(UpdateItem_addr, UpdateItemOther);
    }



    unsafe {
        // PartsSingleModeSkillListItem
        NAMETEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_nameText");
        DESCTEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_descText");
        _BGBUTTON_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_bgButton");
        INFO_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_info");
        set_skill_name_text_addr = get_method_addr(PartsSingleModeSkillListItem, c"SetSkillNameText", 0);

        // PartsSingleModeSkillListItem.Info
        get_IsDrawDesc_addr = get_method_addr(Info, c"get_IsDrawDesc", 0);
        get_IsDrawNeedSkillPoint_addr = get_method_addr(Info, c"get_IsDrawNeedSkillPoint", 0);
        get_Id_addr = get_method_addr(Info, c"get_Id", 0);
    }
}
