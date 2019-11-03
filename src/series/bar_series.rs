use std::collections::{hash_map::IntoIter as HashMapIter, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign};
use std::vec::IntoIter as VecIter;

use crate::chart::ChartContext;
use crate::coord::{DiscreteRanged, Ranged, RangedCoord};
use crate::drawing::DrawingBackend;
use crate::element::{ComposedElement, EmptyElement, Rectangle};
use crate::style::{Color, ShapeStyle, GREEN, TRANSPARENT};

pub trait BarSeriesType {}
#[derive(Debug)]
pub struct Vertical;
#[derive(Debug)]
pub struct Horizontal;

impl BarSeriesType for Vertical {}
impl BarSeriesType for Horizontal {}

/// The series that aggregate data into a bar chart
pub struct BarSeries<'a, BR, A, DataId, Tag = Vertical>
where
    BR: DiscreteRanged,
    BR::ValueType: Eq + Hash + std::fmt::Debug,
    A: AddAssign<A> + Default + std::fmt::Debug,
    Tag: BarSeriesType,
    DataId: Sized + std::fmt::Debug,
{
    style: Box<dyn Fn(&BR::ValueType, &DataId, &A) -> ShapeStyle + 'a>,
    margin: u32,
    iter: HashMapIter<BR::ValueType, Vec<(DataId, A)>>,
    subiter: Option<VecIter<(DataId, A)>>,
    subiter_info: Option<(BR::ValueType, A)>,
    baseline: Box<dyn Fn(BR::ValueType) -> A + 'a>,
    _p: PhantomData<(BR, Tag)>,
}

impl<'a, BR, A, DataId, Tag> BarSeries<'a, BR, A, DataId, Tag>
where
    BR: DiscreteRanged,
    BR::ValueType: Eq + Hash + std::fmt::Debug,
    A: AddAssign<A> + Default + 'a + std::fmt::Debug,
    Tag: BarSeriesType,
    DataId: Sized + std::fmt::Debug,
{
    fn empty() -> Self {
        Self {
            style: Box::new(|_, _, _| GREEN.filled()),
            margin: 5,
            iter: HashMap::new().into_iter(),
            subiter: None,
            subiter_info: None,
            baseline: Box::new(|_| A::default()),
            _p: PhantomData,
        }
    }
    /// Set the style of the bars
    pub fn style<S: Into<ShapeStyle>>(mut self, style: S) -> Self {
        let style = style.into();
        self.style = Box::new(move |_, _, _| style.clone());
        self
    }

    /// Set the style of histogram using a lambda function
    pub fn style_func(
        mut self,
        style_func: impl Fn(&BR::ValueType, &DataId, &A) -> ShapeStyle + 'a,
    ) -> Self {
        self.style = Box::new(style_func);
        self
    }

    /// Set the baseline of the histogram
    pub fn baseline(mut self, baseline: A) -> Self
    where
        A: Clone,
    {
        self.baseline = Box::new(move |_| baseline.clone());
        self
    }

    /// Set a function that defines variant baseline
    pub fn baseline_func(mut self, func: impl Fn(BR::ValueType) -> A + 'a) -> Self {
        self.baseline = Box::new(func);
        self
    }

    /// Set the margin for each bar
    pub fn margin(mut self, value: u32) -> Self {
        self.margin = value;
        self
    }

    /// Set the data iterator
    pub fn data<I: IntoIterator<Item = (BR::ValueType, Vec<(DataId, A)>)>>(mut self, iter: I) -> Self {
        let mut buffer = HashMap::new();
        for (x, y) in iter.into_iter() {
            let entry = buffer.entry(x).or_insert(Vec::new());
            entry.extend(y);
        }
        println!("buffer = {:?}", buffer);
        self.iter = buffer.into_iter();
        self
    }
}

impl<'a, BR, A, DataId> BarSeries<'a, BR, A, DataId, Vertical>
where
    BR: DiscreteRanged,
    BR::ValueType: Eq + Hash + std::fmt::Debug,
    A: AddAssign<A> + Default + 'a + std::fmt::Debug,
    DataId: Sized + std::fmt::Debug,
{
    /// Create a new histogram series.
    ///
    /// - `iter`: The data iterator
    /// - `margin`: The margin between bars
    /// - `style`: The style of bars
    ///
    /// Returns the newly created histogram series
    #[allow(clippy::redundant_closure)]
    pub fn new<S: Into<ShapeStyle>, I: IntoIterator<Item = (BR::ValueType, J)>, J: Iterator<Item = (DataId, A)>>(
        iter: I,
        margin: u32,
        style: S,
    ) -> Self {
        let mut buffer = HashMap::<BR::ValueType, Vec<(DataId, A)>>::new();
        for (x, y) in iter.into_iter() {
            let entry = buffer.entry(x).or_insert(Vec::new());
            entry.append(&mut y.collect());
        }
        let style = style.into();
        Self {
            style: Box::new(move |_, _, _| style.clone()),
            margin,
            iter: buffer.into_iter(),
            subiter: None,
            subiter_info: None,
            baseline: Box::new(|_| A::default()),
            _p: PhantomData,
        }
    }

    pub fn vertical<ACoord, DB>(
        _: &ChartContext<DB, RangedCoord<BR, ACoord>>,
    ) -> Self
    where
        ACoord: Ranged<ValueType = A>,
        DB: DrawingBackend,
    {
        Self::empty()
    }
}

impl<'a, BR, A> BarSeries<'a, BR, A, Horizontal>
where
    BR: DiscreteRanged,
    BR::ValueType: Eq + Hash + std::fmt::Debug,
    A: AddAssign<A> + Default + 'a + std::fmt::Debug,
{
    pub fn horizontal<ACoord, DB>(
        _: &ChartContext<DB, RangedCoord<ACoord, BR>>,
    ) -> Self
    where
        ACoord: Ranged<ValueType = A>,
        DB: DrawingBackend,
    {
        Self::empty()
    }
}

impl<'a, BR, A, DataId> Iterator for BarSeries<'a, BR, A, DataId, Vertical>
where
    BR: DiscreteRanged,
    BR::ValueType: Eq + Hash + Clone + std::fmt::Debug,
    A: Add<A> + AddAssign<A> + Copy + Default + std::fmt::Debug,
    DataId: std::fmt::Debug,
{
    type Item = Rectangle<(BR::ValueType, A)>;
    fn next(&mut self) -> Option<Self::Item> {
        let (new_subiter_info, rect) = if let (Some(subiter), Some((x, base))) = (&mut self.subiter, &self.subiter_info) {
            if let Some((data_id, y_coord)) = subiter.next() {
                let nx = BR::next_value(&x);
                let style = (self.style)(&x, &data_id, &y_coord);
                let mut y_coord = y_coord;
                y_coord += *base;
                let mut rect = Rectangle::new([(x.clone(), y_coord), (nx, *base)], style);
                rect.set_margin(0, 0, self.margin, self.margin);

                (
                    Some((x.clone(), y_coord)),
                    Some(rect),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        self.subiter_info = new_subiter_info;
        if rect.is_some() {
            return rect;
        }

        if let Some((x, y)) = self.iter.next() {
            println!("next iteration: {:?}", (&x, &y));
            let nx = BR::next_value(&x);

            let y_len = y.len();
            return if y_len > 0 {
                let mut y_iter = y.into_iter();
                // With this trick we can avoid the clone trait bound
                let base = (self.baseline)(BR::previous_value(&nx));
                let (data_id, y_coord) = y_iter.next().unwrap();
                let style = (self.style)(&x, &data_id, &y_coord);
                let mut rect = Rectangle::new([(x.clone(), y_coord), (nx, base)], style);
                rect.set_margin(0, 0, self.margin, self.margin);

                if y_len > 1 {
                    self.subiter = Some(y_iter);
                    self.subiter_info = Some((x, y_coord));
                }
                Some(rect)
            } else {
                let mut empty_rect = Rectangle::new([
                        (x, A::default()),
                        (BR::previous_value(&nx), A::default()),
                    ],
                    TRANSPARENT.mix(0.0).filled()
                );
                empty_rect.set_margin(0, 0, 0, 0);
                Some(empty_rect)
            };
        }

        None
    }
}

// TODO: Mirror implementation from Vertical
impl<'a, BR, A, DataId> Iterator for BarSeries<'a, BR, A, DataId, Horizontal>
where
    BR: DiscreteRanged,
    BR::ValueType: Eq + Hash + std::fmt::Debug,
    A: AddAssign<A> + Copy + Default + std::fmt::Debug,
    DataId: Sized + std::fmt::Debug,
{
    type Item = Rectangle<(A, BR::ValueType)>;
    fn next(&mut self) -> Option<Self::Item> {
        /*if let Some((y, x)) = self.iter.next() {
            let ny = BR::next_value(&y);
            // With this trick we can avoid the clone trait bound
            let base = (self.baseline)(BR::previous_value(&ny));
            let style = (self.style)(&y, &x[0].1);
            let mut rect = Rectangle::new([(x[0].1, y), (base, ny)], style);
            rect.set_margin(self.margin, self.margin, 0, 0);
            return Some(rect);
        }*/
        if let Some((y, x)) = self.iter.next() {
            return if !x.is_empty() {
                let ny = BR::next_value(&y);
                // With this trick we can avoid the clone trait bound
                let base = (self.baseline)(BR::previous_value(&ny));
                let (data_id, x_coord) = &x[0];
                let style = (self.style)(&y, data_id, x_coord);
                let mut rect = Rectangle::new([(*x_coord, y), (base, ny)], style);
                rect.set_margin(self.margin, self.margin, 0, 0);
                Some(rect)
            } else {
                None
            };
        }
        None
    }
}
