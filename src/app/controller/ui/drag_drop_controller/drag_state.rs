use super::AppController;

pub(crate) struct DragDropController<'a> {
    controller: &'a mut AppController,
}

impl<'a> DragDropController<'a> {
    pub(crate) fn new(controller: &'a mut AppController) -> Self {
        Self { controller }
    }
}

impl std::ops::Deref for DragDropController<'_> {
    type Target = AppController;

    fn deref(&self) -> &Self::Target {
        self.controller
    }
}

impl std::ops::DerefMut for DragDropController<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.controller
    }
}
