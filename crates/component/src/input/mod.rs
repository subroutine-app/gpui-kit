mod clear_button;
mod content_type;
mod context_menu;
mod input;
mod token;
pub use gpui_base::input::{
    ActivateToken, InlineToken, InlineTokenClickEvent, InlineTokenContext, InlineTokenError,
    InlineTokenHoverEvent, InlineTokenSpan, InputContent,
};
pub use token::InputToken;
pub mod language_config;
mod number_input;
mod otp_input;
mod overlay;
pub(crate) mod popovers;
mod search;
mod syntax_context;

pub(crate) use clear_button::*;
pub use content_type::*;
pub use context_menu::ContextMenuStyle;
#[cfg(not(feature = "tree-sitter"))]
pub struct Tree;
pub use gpui_base::input::CursorBlink;
/// The shared editing engine. Internal to the framework: components reach it
/// through the concrete state of their control, never across the public API.
pub(crate) use gpui_base::input::InputBaseState;
pub use gpui_base::input::{
    Backspace, BufferPoint, CodeActionItem, CodeActionProvider, CompletionHighlighter,
    CompletionMenuOptions, CompletionProvider, Copy, Cut, DefinitionProvider, Delete,
    DeleteToBeginningOfLine, DeleteToEndOfLine, DeleteToNextWordEnd, DeleteToPreviousWordStart,
    DisplayMap, DisplayPoint, DocumentColorProvider, DocumentRangeSemanticTokensProvider,
    EditorState, Enter, Escape, FoldRange, GoToDefinition, GutterDecoration,
    GutterDecorationCollection, HighlightStyleResolver, HoverPopoverState, HoverProvider, Indent,
    IndentInline, InputEdit, InputEditsEvent, InputEvent, InputHighlighter,
    InputHighlighterFactory, InputState, Lsp, MaskPattern, MoveDown, MoveEnd, MoveHome, MoveLeft,
    MovePageDown, MovePageUp, MoveRight, MoveToEnd, MoveToEndOfLine, MoveToNextWord,
    MoveToPreviousWord, MoveToStart, MoveToStartOfLine, MoveUp, Outdent, OutdentInline,
    OverviewRulerDecoration, OverviewRulerDecorationCollection, OverviewRulerLane, Paste, Point,
    PrepaintCaretGeometry, RangeDecoration, RangeDecorationCollection, RangeDecorationStyle, Redo,
    Replace, Rope, RopeExt, RopeLines, Search, SelectAll, SelectToEnd, SelectToEndOfLine,
    SelectToNextWordEnd, SelectToPreviousWordStart, SelectToStart, SelectToStartOfLine, Selection,
    ShowCharacterPalette, ShowCompletions, ShowContextMenu, ShowDocumentHandler,
    SmoothCaretOptions, SubmitKey, TabSize, TextDecoration, TextDecorationCollection,
    TextareaState, ToggleCodeActions, Undo, WrappingIndent, app_smooth_caret, highlight_characters,
    highlight_contains, highlight_prefix, set_app_smooth_caret,
};
pub use gpui_base::input::{EditorMode, InputMode, InputModeKind, TextareaMode};
#[doc(hidden)]
mod editor;
mod group;
mod state;
mod textarea;
pub use editor::Editor;
pub use gpui_base::input::{
    AutoClosingPair, BracketPair, IndentationRules, LanguageProvider, SyntaxContext,
    SyntaxContextProvider, set_language_config, set_language_provider,
};
pub use group::*;
pub use input::*;
pub use lsp_types::Position;
pub use number_input::{NumberInput, NumberInputEvent, NumberStep, StepAction};
pub use otp_input::*;
pub use state::AnyInputState;
pub use textarea::Textarea;

pub(crate) fn init(cx: &mut gpui::App) {
    language_config::init(cx);
}
